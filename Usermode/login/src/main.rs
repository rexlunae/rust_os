// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// GUI root process, handling user logins on a single session
#![feature(const_fn)]

use lazy_static::LazyStatic;

extern crate async;
extern crate lazy_static;
extern crate loader;
extern crate wtk;
#[macro_use]
extern crate syscalls;

macro_rules! imgpath {
		($p:expr) => {concat!("/system/Tifflin/shared/images/",$p)};
}


static VFS_ROOT: LazyStatic< ::syscalls::vfs::Dir > = LazyStatic::new();

fn main()
{
	const MENU_BTN_WIDTH: u32 = 16;
	const MENU_HEIGHT: u32 = 16;
	const ENTRY_FRAME_HEIGHT: u32 = 40;
	const TEXTBOX_HEIGHT: u32 = 16;

	//VFS_ROOT.init(|| ::syscalls::threads::S_THIS_PROCESS.receive_object().unwrap() );

	::wtk::initialise();

	let power_menu = {
		use wtk::menu::{Menu,Entry};
		Menu::new("Power Menu", (
			Entry::new("Restart", 0, "", || kernel_log!("TODO: Restart")),
			Entry::new("Shut Down", 0, "", || kernel_log!("TODO: Shut down")),
			))
		};
	//power_menu.set_pos(, MENU_BTN_WIDTH);

	// Menu bar
	// - Two buttons: Options and power
	let options_icon = ::wtk::image::RasterMonoA::new(imgpath!("options.r8"), ::wtk::Colour::theme_text_bg()).unwrap();
	let power_icon   = ::wtk::image::RasterMonoA::new(imgpath!("power.r8"  ), ::wtk::Colour::theme_text_bg()).unwrap();
	let options_button = ::wtk::Button::new( ::wtk::Image::new(options_icon), |_btn,_win| () );
	let power_button = ::wtk::Button::new(
		::wtk::Image::new(power_icon),
		|_btn,_win| power_menu.show()
		);
	let menubar = ::wtk::StaticBox::new_horiz( (
		::wtk::BoxEle::fixed(MENU_BTN_WIDTH, options_button),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(MENU_BTN_WIDTH, power_button),
		));

	// Login box (vertially stacked, centered)
	let mut username = ::wtk::TextInput::new();
	username.set_shadow("Username");
	
	let mut password = ::wtk::TextInput::new();
	password.set_shadow("Password");
	password.set_obscured('\u{2022}');	// Bullet

	username.bind_submit(|_uname, win| win.tabto(2));
	password.bind_submit(|password, win| {
		//win.hide();
		if let Err(reason) = try_login(&username.get_content(), &password.get_content()) {
			// TODO: Print error to the screen, as an overlay (or another window?)
			kernel_log!("Login failed - {:?}", reason);
			//win.show_message("Login Failed", reason);
		}
		else {
			// try_login also spawns and waits for the shell
		}
		// - Clear username/password and tab back to the username field
		username.clear();
		password.clear();
		win.tabto(1);
		//win.show();
		});

	let loginbox = ::wtk::Frame::new_fat( ::wtk::StaticBox::new_vert((
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed( TEXTBOX_HEIGHT, &username ),
		::wtk::BoxEle::fixed( 1, () ),	// <-- Padding
		::wtk::BoxEle::fixed( TEXTBOX_HEIGHT, &password ),
		::wtk::BoxEle::expand( () ),
		)) );

	let hbox = ::wtk::StaticBox::new_horiz((
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(120, &loginbox),
		::wtk::BoxEle::expand( () ),
		));

	let vbox = ::wtk::StaticBox::new_vert((
		::wtk::BoxEle::fixed( MENU_HEIGHT, &menubar),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(ENTRY_FRAME_HEIGHT, &hbox),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed( MENU_HEIGHT, () ),
		));

	let mut win = ::wtk::Window::new( "Login", &vbox, ::wtk::Colour::theme_body_bg(), () ).expect("Cannot create login window");
	win.maximise();

	win.taborder_add( 1, &username );
	win.taborder_add( 2, &password );

	win.add_shortcut_1( ::syscalls::gui::KeyCode::Application, || power_menu.show() );

	win.focus( &username );

	win.show();

	::async::idle_loop(&mut [
		&mut win,
		&mut power_menu.waiter(),
		]);
}

fn try_login(username: &str, password: &str) -> Result<(), &'static str>
{
	kernel_log!("username = \"{}\", password = \"{}\"", username, password);
	// TODO: Use a proper auth infrastructure
	if username == "root" && password == "password"
	{
		// Start the handle server for this session?
		// - TODO: Should the handle server be per-session, or a global service?
		// - Global service makes some logic easier, but leads to DoS between users
		// Spawn console, and wait for it to terminate
		spawn_console_and_wait("/sysroot/bin/shell");
		Ok( () )
	}
	else
	{
		Err( "Invalid username or password" )
	}
}

fn open_exe(path: &str) -> Result<::syscalls::vfs::File, ::syscalls::vfs::Error> {
	match ::syscalls::vfs::ROOT.open_child_path(path.as_bytes())
	{
	Ok(v) => v.into_file(::syscalls::vfs::FileOpenMode::Execute),
	Err(e) => Err(e),
	}
}

fn spawn_console_and_wait(path: &str)
{
	let handle_server = {
		let path = "/sysroot/bin/handle_server";
		let fh = open_exe(path).unwrap_or_else(|e| panic!("Couldn't open handle server - {:?}", e));
		let pp = loader::new_process(fh, path.as_bytes(), &[]).expect("Could not spawn handle server");
		//pp.send_obj();
		pp.start()
		};
	// TODO: I need something more elegant than this.
	// - Needs to automatically pass the WGH
	// - OR - Just have a wtk method to pass it `::wtk::share_handle(&console)`
	let console = {
		let fh = match open_exe(path)
			{
			Ok(v) => v,
			Err(e) => panic!("Couldn't open executable '{}' - {:?}", path, e),
			};
		let pp = loader::new_process(fh, path.as_bytes(), &[]).expect("Could not spawn shell");
		pp.send_obj( ::syscalls::gui::clone_group_handle() );
		pp.start()
		};
	::syscalls::threads::wait(&mut [console.wait_terminate()], !0);
}

