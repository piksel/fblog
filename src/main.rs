use std::fs;
use std::io;

#[cfg(test)]
extern crate regex;

mod app;
mod config;
mod filter;
mod log;
mod no_color_support;
mod process;
mod substitution;
mod template;

use config::{Config, Options, Profile};
use substitution::Substitution;

fn main() {
  let app = app::app();
  let matches = app.get_matches();

  if let Some(("use-profile", args)) = matches.subcommand() {
    let profile = args.get_one::<String>("profile").expect("required value should be present");
    Config::save_default_profile(profile).unwrap();
    return;
  }

  let mut options = match Config::load_default() {
    Ok(config) => match &config.default_profile {
      Some(p) if p == "default" => config.get_default_profile(),
      None => config.get_default_profile(),
      Some(p) => config.profiles.get(p).expect("default profile not found").clone(),
    },
    Err(config::Error::NoDefault) => Profile::default(),
    Err(e) => panic!("Failed to read config: {}", e),
  }
  .into();

  update_from_matches(&mut options, &matches);

  options.log_settings.add_default_keys();

  let input_filename = matches.get_one::<String>("INPUT").unwrap();
  let mut input = io::BufReader::new(input_read(input_filename));

  let substitution = if options.log_settings.substitution_enabled {
    match Substitution::new(options.log_settings.context_keys.to_vec(), options.log_settings.placeholder_format.clone()) {
      Err(e) => panic!("Invalid placeholder format: {}", e),
      Ok(subst) => Some(subst),
    }
  } else {
    None
  };

  let handlebars = template::fblog_handlebar_registry(&options.template_settings);
  process::process_input(&options, &mut input, &handlebars, substitution.as_ref())
}

fn update_from_matches(options: &mut Options, matches: &clap::ArgMatches) {
  let mut log_settings = &mut options.log_settings;
  if let Some(values) = matches.get_many::<String>("additional-value") {
    log_settings.add_additional_values(values.map(ToOwned::to_owned).collect());
  }

  if let Some(values) = matches.get_many::<String>("message-key") {
    log_settings.add_message_keys(values.map(ToString::to_string).collect());
  }

  if let Some(values) = matches.get_many::<String>("time-key") {
    log_settings.add_time_keys(values.map(ToString::to_string).collect());
  }

  if let Some(values) = matches.get_many::<String>("level-key") {
    log_settings.add_level_keys(values.map(ToString::to_string).collect());
  }

  if let Some(values) = matches.get_many::<String>("context-key") {
    log_settings.substitution_enabled = true;
    log_settings.add_context_keys(values.into_iter().cloned().collect());
  }

  if let Some(value) = matches.get_one::<String>("placeholder-format") {
    log_settings.substitution_enabled = true;
    log_settings.placeholder_format = value.clone();
  }

  log_settings.dump_all = matches.get_flag("dump-all");
  log_settings.with_prefix = matches.get_flag("with-prefix");
  log_settings.print_lua = matches.get_flag("print-lua");

  if let Some(values) = matches.get_many::<String>("excluded-value") {
    log_settings.dump_all = true; // Dump all is implicitly set by exclusion
    log_settings.add_excluded_values(values.map(ToString::to_string).collect());
  }

  if let Some(main_line_format) = matches.get_one::<String>("main-line-format") {
    options.template_settings.main_line_format = main_line_format.clone();
  }

  if let Some(additional_value_format) = matches.get_one::<String>("additional-value-format") {
    options.template_settings.additional_value_format = additional_value_format.clone();
  }

  options.implicit_return = !matches.get_flag("no-implicit-filter-return-statement");
  options.maybe_filter = matches.get_one::<String>("filter").map(ToOwned::to_owned);
}

fn input_read(input_filename: &str) -> Box<dyn io::Read> {
  if input_filename == "-" {
    Box::new(io::stdin())
  } else {
    Box::new(fs::File::open(input_filename).unwrap_or_else(|_| panic!("Can't open file: {}", input_filename)))
  }
}
