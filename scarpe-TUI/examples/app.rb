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
      
      scroll_area max_height: 18 do
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
        
        para "Require File Write Confirmation (yes/no):", stroke: "light_gray"
        file_consent_val = config.fetch("require_file_consent", true) ? "yes" : "no"
        file_consent_input = edit_line(file_consent_val, stroke: "white")
        para ""

        para "Require Bash Exec Confirmation (yes/no):", stroke: "light_gray"
        bash_consent_val = config.fetch("require_bash_consent", true) ? "yes" : "no"
        bash_consent_input = edit_line(bash_consent_val, stroke: "white")
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
            req_file = file_consent_input.text.strip.downcase != "no"
            req_bash = bash_consent_input.text.strip.downcase != "no"
            
            if p.empty? || k.empty? || m.empty? || c.empty?
              error_msg.text = "Please fill in all fields."
            else
              File.write(config_file, JSON.pretty_generate({ 
                "provider" => p, 
                "api_key" => k, 
                "model" => m, 
                "theme_color" => c,
                "require_file_consent" => req_file,
                "require_bash_consent" => req_bash
              }))
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

ENV['API_KEY'] = config["api_key"]
model_name = config["model"]
provider = config["provider"]
theme_color = config["theme_color"] || "cyan"
require_file_consent = config.fetch("require_file_consent", true)
require_bash_consent = config.fetch("require_bash_consent", true)

history_file = File.expand_path("~/.scarpe_ai_history.json")
messages = []

system_prompt = <<~PROMPT
  You are a software engineer integrated into a CLI.
  You have access to the local file system.
  You can read files if the user explicitly asks you to.
  
  If you need to create or modify a file, you MUST use exactly this XML syntax:
  <write_file path="path/to/file.ext">
  code...
  </write_file>
  
  If you need to execute shell commands (e.g., mkdir, mv, ls, npm install), you MUST use exactly this XML syntax:
  <execute_bash>
  command...
  </execute_bash>
PROMPT

if File.exist?(history_file)
  begin
    messages = JSON.parse(File.read(history_file))
  rescue JSON::ParserError
    messages = []
  end
end

if messages.empty? || messages.first["role"] != "system"
  messages.unshift({ "role" => "system", "content" => system_prompt.strip })
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
          files = Dir.glob("#{file_path}/**/*").reject { |f| File.directory?(f) || f.include?(".git") }
          file_contents = files.map do |f|
            begin
              content = File.read(f)
              "--- File: #{f} ---\n#{content}\n"
            rescue
              "--- File: #{f} ---\n[Binary or unreadable]\n"
            end
          end.join("\n")
          
          user_text = "I have analyzed the directory `#{file_path}`. Here are the files:\n\n#{file_contents}\n\nYou can modify or create files (even in new subdirectories)."
          display_text = "Command: Read directory #{file_path} (#{files.count} files)"
        elsif File.exist?(file_path)
          file_content = File.read(file_path)
          user_text = "I have read the file `#{file_path}`. Here is the content:\n\n```\n#{file_content}\n```\n\nAwaiting instructions."
          display_text = "Command: Read #{file_path}"
        else
          user_text = "Cannot find: #{file_path}"
          display_text = user_text
        end
      end

      messages << { "role" => "user", "content" => user_text }
      File.write(history_file, JSON.pretty_generate(messages))

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
            ai_node = para "AI is typing...", stroke: "dark_gray"
            para ""
          end
        end
      end

      Thread.new do
        current_text = ""
        ai_full_response = ""
        first_token_received = false
        uri = nil
        request = nil
        
        begin
          case provider
          when "openai"
            uri = URI("https://api.openai.com/v1/chat/completions")
            request = Net::HTTP::Post.new(uri)
            request['Authorization'] = "Bearer #{ENV['API_KEY']}"
            request.body = { model: model_name, messages: messages.last(20), stream: true }.to_json
          when "openrouter"
            uri = URI("https://openrouter.ai/api/v1/chat/completions")
            request = Net::HTTP::Post.new(uri)
            request['Authorization'] = "Bearer #{ENV['API_KEY']}"
            request.body = { model: model_name, messages: messages.last(20), stream: true }.to_json
          when "anthropic"
            uri = URI("https://api.anthropic.com/v1/messages")
            request = Net::HTTP::Post.new(uri)
            request['x-api-key'] = ENV['API_KEY']
            request['anthropic-version'] = '2023-06-01'
            request.body = { model: model_name, max_tokens: 4096, messages: messages.last(20), stream: true }.to_json
          when "gemini"
            uri = URI("https://generativelanguage.googleapis.com/v1beta/models/#{model_name}:streamGenerateContent?alt=sse&key=#{ENV['API_KEY']}")
            request = Net::HTTP::Post.new(uri)
            gemini_msgs = messages.last(20).reject { |m| m["role"] == "system" }.map do |msg|
              { "role" => msg["role"] == "assistant" ? "model" : "user", "parts" => [{ "text" => msg["content"] }] }
            end
            
            sys_msg = messages.find { |m| m["role"] == "system" }
            body_hash = { contents: gemini_msgs }
            body_hash[:system_instruction] = { parts: [{ text: sys_msg["content"] }] } if sys_msg
            
            request.body = body_hash.to_json
          end

          request['Content-Type'] = 'application/json'
          request['Accept'] = 'text/event-stream'

          Net::HTTP.start(uri.host, uri.port, use_ssl: true, read_timeout: 300, open_timeout: 60) do |http|
            http.request(request) do |response|
              if response.code.to_i == 200
                response.read_body do |chunk|
                  chunk.split("\n").each do |line|
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
                            ai_node.text = ""
                            first_token_received = true
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
                error_data = JSON.parse(response.body) rescue nil
                err_msg = error_data ? (error_data.dig("error", "message") || response.body) : response.body
                ai_full_response = "[API Error #{response.code}] #{err_msg}"
                ai_node.text = current_text + ai_full_response
              end
            end
          end
        rescue StandardError => e
          ai_full_response = "[Network Error] #{e.message}"
          ai_node.text = current_text + ai_full_response
        end
        
        messages << { "role" => "assistant", "content" => ai_full_response }
        File.write(history_file, JSON.pretty_generate(messages))

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
                        dir = File.dirname(path)
                        FileUtils.mkdir_p(dir) unless Dir.exist?(dir)
                        File.write(path, content.strip)
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
              dir = File.dirname(path)
              FileUtils.mkdir_p(dir) unless Dir.exist?(dir)
              File.write(path, content.strip)
              append_to(chat_history_id) do
                para "System: File #{path} saved automatically.", stroke: "green"
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
                        stdout, stderr, _status = Open3.capture3(cmd)
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
                        File.write(history_file, JSON.pretty_generate(messages))
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
              stdout, stderr, _status = Open3.capture3(cmd)
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
              File.write(history_file, JSON.pretty_generate(messages))
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
          button "Exit", stroke: "white", fill: "dark_red" do
            quit
          end
        end
      end
    end
  end
end