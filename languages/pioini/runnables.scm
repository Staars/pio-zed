(
  (section_name
    (text) @env_name
  ) @run
  (#match? @env_name "^env:")
  (#set! tag pio-env)
)
