use adw::prelude::*;
use gtk::{glib, Align, Orientation};
use linuxfo_core::{collect, format_bytes, Snapshot};
use std::{cell::RefCell, rc::Rc, time::Duration};

const CSS:&str=r#"
window { background: #0b1020; color: #e8edf8; }
.sidebar { background: #10182b; border-right: 1px solid #24304a; }
.brand { font-size: 23px; font-weight: 800; letter-spacing: -0.8px; color: #f4f7ff; }
.brand-mark { background: linear-gradient(135deg,#6d7cff,#43d6c5); color: #08101e; border-radius: 12px; padding: 8px; font-weight: 900; }
.nav-button { background: transparent; color: #a9b5cc; border-radius: 10px; padding: 10px 12px; text-align: left; }
.nav-button:hover,.nav-button:checked { background: #202d48; color: #fff; }
.page-title { font-size: 29px; font-weight: 750; letter-spacing: -0.7px; }
.muted { color: #8d9bb6; }
.card { background: #141f34; border: 1px solid #263650; border-radius: 16px; padding: 18px; }
.card-title { color: #a9b8d1; font-size: 12px; font-weight: 700; letter-spacing: 0.7px; }
.metric { font-size: 28px; font-weight: 750; color: #f5f7ff; }
.metric-small { font-size: 17px; font-weight: 650; color: #f5f7ff; }
.accent { color: #69d8c9; }
.pill { background: #1d3b43; color: #7de5d4; border-radius: 20px; padding: 5px 9px; font-size: 11px; }
progressbar trough { background: #263650; min-height: 7px; border-radius: 8px; }
progressbar progress { background: #6d7cff; border-radius: 8px; }
separator { background: #263650; }
"#;

fn label(text:&str, class:&str)->gtk::Label { let l=gtk::Label::new(Some(text));l.set_xalign(0.0);l.add_css_class(class);l }
fn card(title:&str,value:&str,subtitle:&str)->gtk::Box{
 let b=gtk::Box::new(Orientation::Vertical,8);b.add_css_class("card");
 b.append(&label(&title.to_uppercase(),"card-title"));b.append(&label(value,"metric"));b.append(&label(subtitle,"muted"));b
}
fn section(title:&str,subtitle:&str)->gtk::Box{let b=gtk::Box::new(Orientation::Vertical,4);b.append(&label(title,"page-title"));b.append(&label(subtitle,"muted"));b}
fn row_card(title:&str,value:&str,detail:&str)->gtk::Box{let b=gtk::Box::new(Orientation::Horizontal,12);b.add_css_class("card");b.set_hexpand(true);let left=gtk::Box::new(Orientation::Vertical,4);left.set_hexpand(true);left.append(&label(title,"metric-small"));left.append(&label(detail,"muted"));b.append(&left);let v=label(value,"metric-small");v.add_css_class("accent");v.set_valign(Align::Center);b.append(&v);b}
fn content_page(s:&Snapshot, page:&str)->gtk::Widget{
 let root=gtk::Box::new(Orientation::Vertical,18);root.set_margin_top(28);root.set_margin_bottom(28);root.set_margin_start(30);root.set_margin_end(30);
 let title=match page{"CPU"=>("Processor","Architecture, topology and live utilisation"),"Memory"=>("Memory","Physical memory and swap usage"),"Storage"=>("Storage","Filesystems, devices and mount points"),"Thermals"=>("Thermals & Sensors","Kernel-exposed temperature and telemetry channels"),"Cameras"=>("Cameras","Video devices discovered without activating streams"),"Devices"=> ("Hardware Devices","USB and PCI device inventory"),"Network"=>("Network","Interface traffic counters"),_=> ("System Overview","A live view of your Linux machine")};
 root.append(&section(title.0,title.1));
 match page {
 "CPU"=>{root.append(&row_card("Processor",&s.cpu.model,&format!("{} logical CPUs • {} MHz",s.cpu.logical_cores,s.cpu.frequency_mhz)));root.append(&row_card("Current utilisation",&format!("{:.1}%",s.cpu.usage_percent),"Average across logical CPUs"));},
 "Memory"=>{root.append(&row_card("Physical memory",&format!("{} / {}",format_bytes(s.memory.used_bytes),format_bytes(s.memory.total_bytes)),&format!("{} available",format_bytes(s.memory.available_bytes))));root.append(&row_card("Swap",&format!("{} / {}",format_bytes(s.memory.swap_used_bytes),format_bytes(s.memory.swap_total_bytes)),"Kernel swap accounting"));},
 "Storage"=>{for d in &s.disks {let used=d.total_bytes.saturating_sub(d.available_bytes);root.append(&row_card(&d.name,&format_bytes(d.total_bytes),&format!("{} • {} • {} used • {} free{}",d.mount_point,d.filesystem,format_bytes(used),format_bytes(d.available_bytes),if d.removable{" • removable"}else{""})));}},
 "Thermals"=>{if s.sensors.is_empty(){root.append(&row_card("No hwmon channels found","—","This kernel or hardware does not expose readable /sys/class/hwmon inputs."));}else{for t in &s.sensors{root.append(&row_card(&t.label,&format!("{:.1} {}",t.value,t.unit),&format!("{} • {} • {}",t.kind,t.chip,t.source)));}}},
 "Cameras"=>{if s.cameras.is_empty(){root.append(&row_card("No video devices found","—","No /dev/video* nodes were discovered. Check driver and device permissions."));}else{for d in &s.cameras{root.append(&row_card(&d.name,&d.path,&d.detail));}}},
 "Devices"=>{for d in s.usb_devices.iter().chain(s.pci_devices.iter()){root.append(&row_card(&d.name,&d.detail,&d.path));}},
 "Network"=>{for n in &s.networks{root.append(&row_card(&n.name,&format!("↓ {}  ↑ {}",format_bytes(n.received_bytes),format_bytes(n.transmitted_bytes)),"Cumulative interface byte counters"));}},
 _=>{
   let grid=gtk::Grid::new();grid.set_column_spacing(14);grid.set_row_spacing(14);grid.set_column_homogeneous(true);
   let cards=[card("Processor",&format!("{:.0}%",s.cpu.usage_percent),&s.cpu.model),card("Memory",&format!("{:.1}%",if s.memory.total_bytes>0{s.memory.used_bytes as f64*100.0/s.memory.total_bytes as f64}else{0.0}),&format!("{} of {} used",format_bytes(s.memory.used_bytes),format_bytes(s.memory.total_bytes))),card("Storage",&format!("{}",s.disks.len()),"Mounted filesystems discovered"),card("Sensors",&format!("{}",s.sensors.len()),"Readable hwmon channels"),card("Cameras",&format!("{}",s.cameras.len()),"Video devices discovered"),card("Devices",&format!("{}",s.usb_devices.len()+s.pci_devices.len()),"USB and PCI entries")];
   for (i,c) in cards.iter().enumerate(){grid.attach(c,(i%3) as i32,(i/3) as i32,1,1);}root.append(&grid);
   root.append(&row_card("Operating system",&s.host.os,&format!("{} • {}",s.host.architecture,s.host.kernel)));
   root.append(&row_card("Machine",&s.host.hostname,&format!("{} • uptime {}",if s.host.desktop.is_empty(){"Desktop environment unknown"}else{&s.host.desktop},s.host.uptime)));
   root.append(&row_card("CPU model",&s.cpu.model,&format!("{} logical processors",s.cpu.logical_cores)));
   root.append(&label("LIVE SYSTEM SNAPSHOT • AUTO REFRESH EVERY 3 SECONDS","muted"));
 }
 }
 root.upcast()
}
fn main(){
 let app=adw::Application::builder().application_id("io.linuxfo.SystemInfo").build();
 app.connect_activate(|app|{
  let provider=gtk::CssProvider::new();provider.load_from_data(CSS);gtk::style_context_add_provider_for_display(&gtk::gdk::Display::default().expect("display"),&provider,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
  let window=adw::ApplicationWindow::builder().application(app).title("LinuxFo").default_width(1240).default_height(820).build();
  let shell=gtk::Box::new(Orientation::Horizontal,0);
  let sidebar=gtk::Box::new(Orientation::Vertical,14);sidebar.set_width_request(224);sidebar.set_margin_top(24);sidebar.set_margin_bottom(20);sidebar.set_margin_start(16);sidebar.set_margin_end(14);sidebar.add_css_class("sidebar");
  let brand=gtk::Box::new(Orientation::Horizontal,10);let mark=label("LF","brand-mark");brand.append(&mark);brand.append(&label("LinuxFo","brand"));sidebar.append(&brand);sidebar.append(&label("YOUR SYSTEM, IN FOCUS","muted"));
  let stack=gtk::Stack::new();stack.set_hexpand(true);stack.set_vexpand(true);stack.set_transition_type(gtk::StackTransitionType::Crossfade);
  let names=["Overview","CPU","Memory","Storage","Thermals","Cameras","Devices","Network"];
  let buttons=gtk::Box::new(Orientation::Vertical,4);let first=gtk::CheckButton::new();first.set_label("◫   Overview");first.set_active(true);first.add_css_class("nav-button");buttons.append(&first);
  let current=Rc::new(RefCell::new(String::from("Overview")));
  for name in names.iter().skip(1){let btn=gtk::CheckButton::new();btn.set_label(&format!("{}   {}",match *name{"CPU"=>"◉","Memory"=>"▤","Storage"=>"▣","Thermals"=>"⌁","Cameras"=>"▣","Devices"=>"◈",_=>"⌘"},name));btn.set_group(Some(&first));btn.add_css_class("nav-button");let c=current.clone();let st=stack.clone();btn.connect_toggled(move|b|{if b.is_active(){*c.borrow_mut()=name.to_string();st.set_visible_child_name(name);}});buttons.append(&btn);}
  first.connect_toggled({let c=current.clone();let st=stack.clone();move|b|if b.is_active(){*c.borrow_mut()="Overview".into();st.set_visible_child_name("Overview");}});
  sidebar.append(&buttons);sidebar.append(&gtk::Separator::new(Orientation::Horizontal));sidebar.append(&label("LINUXFO  •  0.1.0","muted"));
  let main=gtk::Box::new(Orientation::Vertical,0);main.set_hexpand(true);let top=gtk::Box::new(Orientation::Horizontal,12);top.set_margin_top(14);top.set_margin_bottom(10);top.set_margin_start(30);top.set_margin_end(30);top.append(&label("SYSTEM INTELLIGENCE","muted"));let status=label("●  LIVE","pill");top.append(&status);top.set_halign(Align::Fill);main.append(&top);
  let snapshot=Rc::new(RefCell::new(collect()));
  for name in names {let page=content_page(&snapshot.borrow(),name);stack.add_named(&page,Some(name));}
  let scroller=gtk::ScrolledWindow::new();scroller.set_vexpand(true);scroller.set_hexpand(true);scroller.set_child(Some(&stack));main.append(&scroller);
  shell.append(&sidebar);shell.append(&main);window.set_content(Some(&shell));window.present();
  let snap=snapshot.clone();let st=stack.clone();glib::timeout_add_local(Duration::from_secs(3),move||{*snap.borrow_mut()=collect();let name=st.visible_child_name().map(|n|n.to_string()).unwrap_or_else(||"Overview".into());st.set_visible_child(Some(&content_page(&snap.borrow(),&name)));glib::ControlFlow::Continue});
 });app.run();
}
