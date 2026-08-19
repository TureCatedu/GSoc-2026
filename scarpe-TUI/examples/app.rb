require_relative '../lib/scarpe_tui'
require 'net/http'
require 'json'
require 'fileutils'
require 'open3'

config_file = File.expand_path("~/.scarpe_ai_config.json")
config = {}

if File.exist?(config_file)
  begin
    config = JSON.parse(File.read(config_file))
  rescue JSON::ParserError
    config = {}
  end
end

force_setup = ARGV.include?("--setup")

if force_setup || config["api_key"].nil? || config["api_key"].empty? || config["model"].nil? || config["model"].empty? || config["provider"].nil? || config["provider"].empty?
  Scarpe.app(true, title: "Scarpe AI Setup") do
    provider_input = nil
    api_input = nil
    model_input = nil
    color_input = nil
    file_consent_input = nil
    bash_consent_input = nil
    error_msg = nil

    border stroke: "blue" do
      stack do
        para " SCARPE AI - SETUP ", stroke: "white", fill: "blue", modifier: "bold"
        para ""
      end

      scroll_area max_height: 22 do
        para ""
        para "Provider (openai, anthropic, gemini, openrouter):", stroke: "light_gray"
        provider_input = edit_line(config["provider"] || "openai", stroke: "white")
        para ""

        para "API Key:", stroke: "light_gray"
        api_input = edit_line(config["api_key"] || "", stroke: "white")
        para ""

        para "Model:", stroke: "light_gray"
        model_input = edit_line(config["model"] || "gpt-4o", stroke: "white")
        para ""

        para "Theme Color (cyan, green, magenta, yellow, white):", stroke: "light_gray"
        color_input = edit_line(config["theme_color"] || "cyan", stroke: "white")
        para ""

        para "Require File Write Confirmation:", stroke: "light_gray"
        file_consent_input = checkbox "Ask before creating or modifying files"
        file_consent_input.checked = config.fetch("require_file_consent", true)
        para ""

        para "Require Bash Exec Confirmation:", stroke: "light_gray"
        bash_consent_input = checkbox "Ask before executing shell commands"
        bash_consent_input.checked = config.fetch("require_bash_consent", true)
        para ""

        error_msg = para "", stroke: "red"
      end

      dock_bottom do
        para " "
        flow do
          button "SAVE", stroke: "white", fill: "dark_blue" do
            p = provider_input.text.strip.downcase
            k = api_input.text.strip
            m = model_input.text.strip
            c = color_input.text.strip
            req_file = file_consent_input.checked?
            req_bash = bash_consent_input.checked?

            if p.empty? || k.empty? || m.empty? || c.empty?
              error_msg.text = "Please fill in all fields."
            else
              File.open(config_file, File::WRONLY | File::CREAT | File::TRUNC, 0o600) do |file|
                file.write(JSON.pretty_generate({
                  "provider" => p,
                  "api_key" => k,
                  "model" => m,
                  "theme_color" => c,
                  "require_file_consent" => req_file,
                  "require_bash_consent" => req_bash
                }))
              end
              File.chmod(0o600, config_file)
              quit
            end
          end

          button "EXIT", stroke: "white", fill: "dark_red" do
            quit
          end
        end
      end
    end
  end

  if File.exist?(config_file)
    config = JSON.parse(File.read(config_file))
  else
    exit
  end
end

api_key = config.fetch("api_key")
model_name = config["model"]
provider = config["provider"]
theme_color = config["theme_color"] || "cyan"
require_file_consent = config.fetch("require_file_consent", true)
require_bash_consent = config.fetch("require_bash_consent", true)

safe_command_env = {
  "PATH" => ENV.fetch("PATH", ""),
  "HOME" => ENV.fetch("HOME", ""),
  "LANG" => ENV.fetch("LANG", "C.UTF-8")
}.freeze

workspace_root = File.realpath(Dir.pwd)

def safe_workspace_path(workspace_root, requested_path)
  path = requested_path.to_s.strip
  raise ArgumentError, "file path must not be empty" if path.empty?

  candidate = File.expand_path(path, workspace_root)
  root = workspace_root.end_with?(File::SEPARATOR) ? workspace_root : "#{workspace_root}#{File::SEPARATOR}"
  unless candidate == workspace_root || candidate.start_with?(root)
    raise ArgumentError, "file path must stay inside the current workspace"
  end
  candidate
end

def write_private_file(path, content)
  File.open(path, File::WRONLY | File::CREAT | File::TRUNC, 0o600) do |file|
    file.write(content)
  end
  File.chmod(0o600, path)
end

history_file = File.expand_path("~/.scarpe_ai_history.json")
messages = []

system_prompt = <<~PROMPT
  You are an expert AI software engineer integrated directly into the user's terminal CLI.
  You DO NOT have a simulated environment. You are running natively on the user's machine.
  
  CRITICAL RULES FOR ACTING ON THE SYSTEM:
  You must NEVER say "I cannot run commands" or "I cannot write files". You HAVE the tools to do so.
  To use these tools, you MUST wrap your output in the specific XML tags defined below.
  Do not use markdown blocks (```bash) to execute commands. ONLY use the XML tags.

  TOOL 1: WRITE OR MODIFY FILES
  If you need to create, update, or write code to a file, use exactly this syntax:
  <write_file path="/absolute/or/relative/path/to/file.ext">
  print("Hello World")
  </write_file>

  TOOL 2: EXECUTE BASH COMMANDS
  If you need to move files, create directories, run scripts, or perform any terminal action, use exactly this syntax:
  <execute_bash>
  mkdir -p /path/to/dir && mv file.txt /path/to/dir/
  </execute_bash>

  When the user asks you to do something, simply execute it using the tools. Do not ask for permission, the system handles it.
PROMPT

def build_provider_context(messages, limit: 20)
  system_message = messages.find { |message| message["role"] == "system" }
  conversation = messages.reject { |message| message["role"] == "system" }.last(limit)
  [system_message, conversation]
end

if File.exist?(history_file)
  begin
    messages = JSON.parse(File.read(history_file))
  rescue JSON::ParserError
    messages = []
  end
end

if messages.empty? || messages.first["role"] != "system"
  messages.unshift({ "role" => "system", "content" => system_prompt.strip })
else
  messages.first["content"] = system_prompt.strip
end

Scarpe.app(true, title: "Scarpe AI") do
  prompt_input = nil
  chat_history_id = nil

  send_logic = proc do
    user_text = prompt_input.text.strip
    if !user_text.empty?

      if user_text == "/setup" || user_text == "/config"
        if $0.end_with?(".rb")
          exec("ruby", $0, "--setup")
        else
          exec($0, "--setup")
        end
      end

      if user_text == "/clear"
        File.delete(history_file) if File.exist?(history_file)
        if $0.end_with?(".rb")
          exec("ruby", $0)
        else
          exec($0)
        end
      end

      display_text = user_text
      if user_text.start_with?("/read ")
        file_path = user_text.sub("/read ", "").strip

        if File.directory?(file_path)
          ignore_dirs = [".git", "node_modules", "target", "build", "vendor", "dist", ".next", ".idea"]

          files = Dir.glob("#{file_path}/**/*").reject do |f|
            File.directory?(f) || ignore_dirs.any? { |ig| f.include?("/#{ig}/") || f.start_with?("#{ig}/") }
          end

          file_contents = files.map do |f|
            if File.size(f) > 150_000
              "--- File: #{f} ---\n[Skipped: File too large (>150KB)]\n"
            else
              begin
                content = File.read(f, encoding: 'UTF-8', invalid: :replace, undef: :replace, replace: '')
                if content.include?("\u0000")
                  "--- File: #{f} ---\n[Skipped: Binary file]\n"
                else
                  "--- File: #{f} ---\n#{content}\n"
                end
              rescue
                "--- File: #{f} ---\n[Unreadable file]\n"
              end
            end
          end.join("\n")

          user_text = "I have analyzed the directory `#{file_path}`. Here are the files:\n\n#{file_contents}\n\nYou can modify or create files (even in new subdirectories)."
          display_text = "Command: Read directory #{file_path} (#{files.count} files)"
        elsif File.exist?(file_path)
          begin
            file_content = File.read(file_path, encoding: 'UTF-8', invalid: :replace, undef: :replace, replace: '')
            user_text = "I have read the file `#{file_path}`. Here is the content:\n\n```\n#{file_content}\n```\n\nAwaiting instructions."
          rescue
            user_text = "I could not read the file `#{file_path}` due to an encoding error."
          end
          display_text = "Command: Read #{file_path}"
        else
          user_text = "Cannot find: #{file_path}"
          display_text = user_text
        end
      end

      messages << { "role" => "user", "content" => user_text }
      write_private_file(history_file, JSON.pretty_generate(messages))

      append_to(chat_history_id) do
        border stroke: theme_color do
          stack do
            para " USER ", stroke: "white", fill: theme_color, modifier: "bold"
            para display_text, stroke: "white"
            para ""
          end
        end
      end

      prompt_input.text = ""

      ai_node = nil
      append_to(chat_history_id) do
        border stroke: "dark_gray" do
          stack do
            para " AI ", stroke: "white", fill: "dark_gray", modifier: "bold"
            ai_node = para "⠋ Thinking...", stroke: "white"
            para ""
          end
        end
      end

      Thread.new do
        current_text = ""
        ai_full_response = ""
        first_token_received = false

        spinner_thread = Thread.new do
          frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
          i = 0
          while !first_token_received
            ai_node.text = "#{frames[i % frames.length]} Thinking..."
            i += 1
            sleep 0.08
          end
        end

        uri = nil
        request = nil

        begin
          system_message, conversation = build_provider_context(messages, limit: 20)

          case provider
          when "openai", "openrouter"
            uri = URI(
              provider == "openai" ?
                "https://api.openai.com/v1/chat/completions" :
                "https://openrouter.ai/api/v1/chat/completions"
            )
            request = Net::HTTP::Post.new(uri)
            request["x-goog-api-key"] = api_key
            request["Authorization"] = "Bearer #{api_key}"
            provider_messages = []
            provider_messages << system_message if system_message
            provider_messages.concat(conversation)

            request.body = {
              model: model_name,
              messages: provider_messages,
              stream: true
            }.to_json

          when "anthropic"
            uri = URI("https://api.anthropic.com/v1/messages")
            request = Net::HTTP::Post.new(uri)
            request["x-api-key"] = api_key
            request["anthropic-version"] = "2023-06-01"

            anthropic_messages = conversation.drop_while do |message|
              message["role"] != "user"
            end

            anthropic_body = {
              model: model_name,
              max_tokens: 4096,
              messages: anthropic_messages,
              stream: true
            }
            anthropic_body[:system] = system_message["content"] if system_message
            request.body = anthropic_body.to_json

          when "gemini"
            uri = URI(
              "https://generativelanguage.googleapis.com/" \
              "v1beta/models/#{model_name}:streamGenerateContent?alt=sse"
            )
            request = Net::HTTP::Post.new(uri)
            request["x-goog-api-key"] = api_key

            gemini_messages = conversation.map do |message|
              {
                "role" => message["role"] == "assistant" ? "model" : "user",
                "parts" => [{ "text" => message["content"] }]
              }
            end

            body_hash = { contents: gemini_messages }
            if system_message
              body_hash[:system_instruction] = {
                parts: [{ text: system_message["content"] }]
              }
            end
            request.body = body_hash.to_json
          end

          request['Content-Type'] = 'application/json'
          request['Accept'] = 'text/event-stream'

          Net::HTTP.start(uri.host, uri.port, use_ssl: true, read_timeout: 300, open_timeout: 60) do |http|
            http.request(request) do |response|
              if response.code.to_i == 200
                sse_buffer = +""
                response.read_body do |chunk|
                  sse_buffer << chunk
                  complete_lines = sse_buffer.split("\n", -1)
                  sse_buffer = complete_lines.pop.to_s

                  complete_lines.each do |line|
                    if line.start_with?("data: ") && line != "data: [DONE]"
                      data_str = line.sub("data: ", "").strip
                      next if data_str.empty?

                      begin
                        json = JSON.parse(data_str)
                        content = nil

                        case provider
                        when "openai", "openrouter"
                          content = json.dig("choices", 0, "delta", "content")
                        when "anthropic"
                          content = json.dig("delta", "text") if json["type"] == "content_block_delta"
                        when "gemini"
                          content = json.dig("candidates", 0, "content", "parts", 0, "text")
                        end

                        if content
                          if !first_token_received
                            first_token_received = true
                            spinner_thread.kill if spinner_thread.alive?
                            ai_node.text = ""
                          end

                          current_text += content
                          ai_full_response += content
                          ai_node.text = current_text
                        end
                      rescue JSON::ParserError
                      end
                    end
                  end
                end
              else
                first_token_received = true
                spinner_thread.kill if spinner_thread.alive?
                error_data = JSON.parse(response.body) rescue nil
                err_msg = error_data ? (error_data.dig("error", "message") || response.body) : response.body
                ai_full_response = "[API Error #{response.code}] #{err_msg}"
                ai_node.text = current_text + ai_full_response
              end
            end
          end
        rescue StandardError => e
          first_token_received = true
          spinner_thread.kill if spinner_thread.alive?
          ai_full_response = "[Network Error] #{e.message}"
          ai_node.text = current_text + ai_full_response
        end

        messages << { "role" => "assistant", "content" => ai_full_response }
        write_private_file(history_file, JSON.pretty_generate(messages))

        ai_full_response.scan(/<write_file path="([^"]+)">(.*?)<\/write_file>/m).each do |path, content|
          if require_file_consent
            append_to(chat_history_id) do
              border stroke: "yellow" do
                stack do
                  para " The AI wants to create/modify: #{path} ", stroke: "black", fill: "yellow", modifier: "bold"
                  status_msg = para "Waiting for confirmation...", stroke: "light_gray"

                  flow do
                    button "Allow", stroke: "white", fill: "dark_green" do
                      begin
                        safe_path = safe_workspace_path(workspace_root, path)
                        dir = File.dirname(safe_path)
                        FileUtils.mkdir_p(dir) unless Dir.exist?(dir)
                        write_private_file(safe_path, content.strip)
                        status_msg.text = "File saved successfully."
                      rescue StandardError => e
                        status_msg.text = "Error: #{e.message}"
                      end
                    end

                    button "Reject", stroke: "white", fill: "dark_red" do
                      status_msg.text = "Operation cancelled."
                    end
                  end
                end
              end
              para " "
              para " "
              para " "
            end
          else
            begin
              safe_path = safe_workspace_path(workspace_root, path)
              dir = File.dirname(safe_path)
              FileUtils.mkdir_p(dir) unless Dir.exist?(dir)
              write_private_file(safe_path, content.strip)
              append_to(chat_history_id) do
                para "System: File #{safe_path} saved automatically.", stroke: "green"
              end
            rescue StandardError => e
              append_to(chat_history_id) do
                para "System: Error saving #{path} - #{e.message}", stroke: "red"
              end
            end
          end
        end

        ai_full_response.scan(/<execute_bash>\s*(.*?)\s*<\/execute_bash>/m).each do |command|
          cmd = command[0].strip
          if require_bash_consent
            append_to(chat_history_id) do
              border stroke: "yellow" do
                stack do
                  para " The AI wants to execute: ", stroke: "black", fill: "yellow", modifier: "bold"
                  para cmd, stroke: "white"
                  status_msg = para "Waiting for confirmation...", stroke: "light_gray"

                  flow do
                    button "Allow", stroke: "white", fill: "dark_green" do
                      begin
                        stdout, stderr, _status = Open3.capture3(safe_command_env, cmd, unsetenv_others: true)
                        output = stdout.strip.empty? ? stderr.strip : stdout.strip
                        output = "Success (no output)." if output.empty?

                        status_msg.text = "Execution completed."

                        append_to(chat_history_id) do
                          border stroke: "dark_gray" do
                            stack do
                              para " SYSTEM ", stroke: "white", fill: "dark_gray", modifier: "bold"
                              para "Result for `#{cmd}`:\n#{output[0..1000]}", stroke: "light_gray"
                              para ""
                            end
                          end
                        end

                        messages << { "role" => "user", "content" => "System observation - Bash execution result for `#{cmd}`:\n#{output}" }
                        write_private_file(history_file, JSON.pretty_generate(messages))
                      rescue StandardError => e
                        status_msg.text = "Error: #{e.message}"
                      end
                    end

                    button "Reject", stroke: "white", fill: "dark_red" do
                      status_msg.text = "Execution cancelled."
                    end
                  end
                end
              end
              para " "
              para " "
              para " "
            end
          else
            begin
              stdout, stderr, _status = Open3.capture3(safe_command_env, cmd, unsetenv_others: true)
              output = stdout.strip.empty? ? stderr.strip : stdout.strip
              output = "Success (no output)." if output.empty?

              append_to(chat_history_id) do
                border stroke: "dark_gray" do
                  stack do
                    para " SYSTEM ", stroke: "white", fill: "dark_gray", modifier: "bold"
                    para "Executed `#{cmd}`.\nResult:\n#{output[0..1000]}", stroke: "light_gray"
                    para ""
                  end
                end
              end

              messages << { "role" => "user", "content" => "System observation - Bash execution result for `#{cmd}`:\n#{output}" }
              write_private_file(history_file, JSON.pretty_generate(messages))
            rescue StandardError => e
              append_to(chat_history_id) do
                para "System: Error executing #{cmd} - #{e.message}", stroke: "red"
              end
            end
          end
        end

      end
    end
  end

  stack do
    para " SCARPE AI ", stroke: "white", fill: theme_color, modifier: "bold"
    para " Use /read <path> to inject files. /setup to configure.", stroke: "dark_gray"
    para ""
  end

  chat_history_id = scroll_area max_height: 0 do
    messages.each do |msg|
      next if msg["role"] == "system"

      is_user = msg["role"] == "user"
      b_stroke = is_user ? theme_color : "dark_gray"
      bg_color = is_user ? theme_color : "dark_gray"
      label = is_user ? " USER " : " AI "

      border stroke: b_stroke do
        stack do
          para label, stroke: "white", fill: bg_color, modifier: "bold"

          if msg["content"].start_with?("I have read the file") || msg["content"].start_with?("I have analyzed the directory")
            para "Command: Read operation completed.", stroke: "light_gray"
          elsif msg["content"].start_with?("System observation - Bash execution result")
            para msg["content"], stroke: "light_gray"
          else
            para msg["content"], stroke: "white"
          end
          para ""
        end
      end
    end
  end

  dock_bottom do
    border stroke: theme_color do
      stack do
        prompt_input = edit_box("", stroke: "white", &send_logic)
        flow do
          button "Send", stroke: "white", fill: theme_color, &send_logic
          button "Page Up", stroke: "white", fill: theme_color do
            scroll_to_start
          end
          button "Page Down", stroke: "white", fill: theme_color do
            scroll_to_end
          end
          button "Exit", stroke: "white", fill: "dark_red" do
            quit
          end
        end
      end
    end
  end
end