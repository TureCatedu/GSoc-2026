require_relative '../lib/scarpe_tui'

Scarpe.app(true, title: "Test Finale Scarpe-TUI") do
  border stroke: "cyan" do
    
    stack do
      para "=== GENERATORE DI PROFILI GSoC 2026 ===", stroke: "cyan", modifier: "bold"
      para "Compila i campi qui sotto con la tastiera", stroke: "blue"
      para "e usa il MOUSE per cliccare i bottoni."
      para "----------------------------------------"
    end

    para "Come ti chiami?"
    nome_input = edit_line("", stroke: "dark_yellow")

    para "Qual e' il tuo linguaggio preferito?"
    lang_input = edit_line("", stroke: "dark_yellow")

    para " "

    # Pre-dichiariamo la variabile per renderla visibile in tutta l'app
    termini_checkbox = nil 
    border stroke: "magenta" do
      flow do
        termini_checkbox = checkbox stroke: "magenta"
        para " Accetto i termini e condizioni"
      end
    end

    # Pre-dichiariamo anche bio_input nello scope principale!
    bio_input = nil
    
    border stroke: "blue" do
      stack do
        para "Scrivi una breve biografia (Usa INVIO per andare a capo):"
        bio_input = edit_box("") # Ora modifichiamo la variabile pre-dichiarata
      end
    end

    para " "

    flow do
      button "Genera Profilo", stroke: "white", fill: "dark_green" do
        
        nome = nome_input.text.strip
        linguaggio = lang_input.text.strip
        biografia = bio_input.text.strip
        
        accettato = termini_checkbox.checked?

        if nome.empty? || linguaggio.empty?
          para "-> ERRORE: Compila tutti i campi prima di generare!", stroke: "red"
        elsif !accettato
          para "-> ERRORE: Devi accettare i termini e condizioni per proseguire!", stroke: "red"
        else
          para " "
          para "*** NUOVO PROFILO GENERATO ***", stroke: "green", modifier: "bold"
          para "Nome Utente: #{nome}"
          para "Linguaggio : #{linguaggio}"
          para "Qualifica  : Architetto di Interfacce Native"
          para "Termini    : Accettati"
          para "Biografia  :"
          
          if biografia.empty?
            para "Nessuna biografia inserita.", stroke: "dark_gray"
          else
            para biografia, stroke: "dark_yellow", modifier: "italic"
          end
          
          para "******************************", stroke: "green"
        end
      end

      button "Esci", stroke: "white", fill: "dark_red" do
        quit
      end
    end

  end
end