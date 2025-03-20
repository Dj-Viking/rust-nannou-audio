use ringbuf::traits::{ Consumer, Producer, Split };
use ringbuf::HeapRb;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use nannou::prelude::*;
use nannou_audio as audio;
use nannou_audio::Buffer;

struct Model {
	audio_thread: JoinHandle<()>,
	audio_tx: Sender<AudioCommand>,
	is_paused: bool,
}

fn main() {
	nannou::app(model).run();

}
pub enum AudioCommand {
	Play,
	Pause,
	Exit,
}

struct InputModel {
	pub producer: ringbuf::HeapProd<f32>,
}
struct OutputModel {
	pub consumer: ringbuf::HeapCons<f32>,
}

fn model(app: &App) -> Model {
	app.new_window()
		.key_pressed(key_pressed)
		.view(view)
		.build();

	let audio_host = audio::Host::new();
	println!("default input dev {:?}", audio_host.default_input_device().unwrap().name().unwrap());

	const LATENCY: usize = 2048;
	let ringbuffer = HeapRb::<f32>::new(LATENCY * 2);

	let (mut prod, cons) = ringbuffer.split();

	for _ in 0..LATENCY as usize {
		prod.try_push(0.0).unwrap();
	}

	let (audio_tx, audio_rx) = std::sync::mpsc::channel();
	let audio_thread = std::thread::spawn(move || {

		let in_model = InputModel { producer: prod };
		let in_stream = audio_host
			.new_input_stream(in_model)
			.capture(pass_in)
			.build()
			.unwrap();

		let out_model = OutputModel { consumer: cons };
		let out_stream = audio_host
			.new_output_stream(out_model)
			.render(pass_out)
			.build()
			.unwrap();

		let in_stream = in_stream;
		let out_stream = out_stream;
		loop {
			match audio_rx.recv() {
				Ok(AudioCommand::Play) => {
					in_stream.play().unwrap();
					out_stream.play().unwrap();
				},
				Ok(AudioCommand::Pause) => {
					in_stream.pause().unwrap();
					out_stream.pause().unwrap();
				},
				Ok(AudioCommand::Exit) => {
					in_stream.pause().ok();
					out_stream.pause().ok();
					break
				},
				Err(_) => break,
			}
		}
	});
	

	Model {
		audio_thread,
		audio_tx,
		is_paused: false,
	}
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
	match key {
		Key::Space if model.is_paused => {
			model.audio_tx.send(AudioCommand::Play).ok();
			model.is_paused = false;
		},
		Key::Space if !model.is_paused => {
			model.audio_tx.send(AudioCommand::Pause).ok();
			model.is_paused = true;
		},
		_ => ()
	}
}

fn pass_in(model: &mut InputModel, buffer: &Buffer) {
	buffer.frames().for_each(|f| f.into_iter().for_each(|s| {
		let _ = model.producer.try_push(*s);
	}))
}
fn pass_out(model: &mut OutputModel, buffer: &mut Buffer) {
	buffer.frames_mut().for_each(|f| 
		f.iter_mut().for_each(|s| 
			*s = model.consumer.try_pop().unwrap_or(0.0)))
}


fn view(app: &App, _: &Model, frame: Frame) {
	let draw = app.draw();
	draw.background().color(DIMGRAY);
}
