require_relative '../lib/scarpe_tui'

Scarpe.app(true, title: "Test Finale Scarpe-TUI") do
  
  stack do
    para "=== GENERATORE DI PROFILI GSoC 2026 ==="
    para "Compila i campi qui sotto con la tastiera"
    para "e usa il MOUSE per cliccare i bottoni."
    para "----------------------------------------"
  end

  para "Come ti chiami?"

  nome_input = edit_line("")

  para "Qual e' il tuo linguaggio preferito?"
  lang_input = edit_line("")

  para " "

  flow do
    button "Genera Profilo" do

      nome = nome_input.text.strip
      linguaggio = lang_input.text.strip

      if nome.empty? || linguaggio.empty?
        para "-> ERRORE: Compila tutti i campi prima di generare!"
      else
        para " "
        para "*** NUOVO PROFILO GENERATO ***"
        para "Nome Utente: #{nome}"
        para "Linguaggio : #{linguaggio}"
        para "Qualifica  : Architetto di Interfacce Native"
        para "******************************"
      end
    end

    button "Esci" do
      quit
    end
  end

end