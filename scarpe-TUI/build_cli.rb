require 'base64'
require 'fileutils'

dylib_path = "rust_core/target/release/librust_core.dylib"
mylib_path = "ext/mylib.rb"
scarpe_tui_path = "lib/scarpe_tui.rb"
app_path = "examples/app.rb"

unless File.exist?(dylib_path)
  puts "Error: Compile Rust project first with `cd rust_core && cargo build --release`."
  exit 1
end

puts "Bundling Scarpe AI CLI..."

dylib_b64 = Base64.encode64(File.binread(dylib_path))
dylib_size = File.size(dylib_path)

mylib_src = File.read(mylib_path)
  .gsub(/require 'ffi'/, "")
  .gsub(/lib_path\s*=\s*.*/, "lib_path = DYLIB_PATH")

scarpe_tui_src = File.read(scarpe_tui_path)
  .gsub(/require_relative .*/, "")

app_src = File.read(app_path)
  .gsub(/require_relative .*/, "")
  .gsub(/require 'net\/http'/, "")
  .gsub(/require 'json'/, "")
  .gsub(/require 'fileutils'/, "")
  .gsub(/exec\("ruby \#\{__FILE__\}\"\)/, "exec(__FILE__)")

template = <<~RUBY
#!/usr/bin/env ruby

begin
  require 'ffi'
  require 'net/http'
  require 'json'
  require 'fileutils'
  require 'base64'

  CLI_DIR = File.expand_path("~/.scarpe_ai")
  DYLIB_PATH = File.join(CLI_DIR, "librust_core.dylib")
  FileUtils.mkdir_p(CLI_DIR)

  unless File.exist?(DYLIB_PATH) && File.size(DYLIB_PATH) == #{dylib_size}
    payload = DATA.read
    File.binwrite(DYLIB_PATH, Base64.decode64(payload))
    
    if RUBY_PLATFORM =~ /darwin/
      system("codesign -s - -f '\#{DYLIB_PATH}' >/dev/null 2>&1")
    end
  end

#{mylib_src}
#{scarpe_tui_src}
#{app_src}

rescue Exception => e
  log_file = File.expand_path("~/.scarpe_ai/error.log")
  File.write(log_file, "\#{e.class}: \#{e.message}\\n\#{e.backtrace.join("\\n")}")
  puts "Scarpe AI encountered a fatal error."
  puts "Check the log file at: \#{log_file}"
end

__END__
#{dylib_b64}
RUBY

File.write("scarpe", template)
FileUtils.chmod("+x", "scarpe")

puts "Executable 'scarpe' generated successfully!"