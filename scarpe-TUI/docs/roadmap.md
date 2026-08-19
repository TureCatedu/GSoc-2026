# Scarpe TUI Roadmap

## Completato

- Backend Rust collegato alla DSL Ruby tramite FFI.
- Buffer terminale a doppio livello e rendering incrementale.
- Layout ricorsivo per root, stack, flow, border, dock bottom e scroll area.
- Input da tastiera e mouse.
- Editor Unicode per `EditLine` e `EditBox`.
- Movimento del cursore tra righe e mantenimento della colonna verticale.
- Inserimento di newline nell'`EditBox` con `Shift+Enter`.
- Fallback per terminali che espongono la combinazione come `Alt+Enter` o `Ctrl+Enter`.
- Invio con `Enter` normale.
- Checkbox native per le impostazioni di consenso nella schermata di setup.
- Persistenza dei consensi come valori booleani.
- Scrolling con `Up`/`Down` nelle viste scrollabili quando nessun editor è focalizzato.
- Scrolling con `Page Up` e `Page Down`.
- Pulsanti e API Ruby `scroll_to_start` / `scroll_to_end`.
- Scrollbar verticale con track e thumb nella `ScrollArea`.
- Setup iniziale con il prompt del provider visibile nella viewport.
- Test Rust per editor e buffer.
- Build debug e release verificate.
- Sintassi Ruby e controllo del diff verificati.

## Fase corrente

Consolidamento del percorso di distribuzione:

1. compilare il backend Rust in release;
2. rigenerare l'eseguibile bundled;
3. verificare che il bundle contenga la libreria release aggiornata;
4. mantenere separati gli artefatti di build dalle modifiche sorgenti.

## Stato dello scrolling

Lo scrolling delle `ScrollArea` è completo per il percorso corrente:

- `Up`/`Down` e rotella del mouse modificano l'offset della vista;
- `Page Up` e `Page Down` portano la vista rispettivamente verso l'inizio e la fine;
- `scroll_to_start` e `scroll_to_end` espongono lo stesso controllo ai callback Ruby;
- il rendering disegna una track e un thumb verticali quando il contenuto supera la viewport.

Restano possibili miglioramenti UX, come focus più evidente e test aggiuntivi per casi limite di resize e contenuti annidati.

## Prossime fasi

### Fase 1 — Distribuzione

- rigenerare il bundle release;
- verificare il caricamento FFI da sorgente e da bundle;
- aggiungere una verifica automatica non interattiva del bundle.

### Fase 2 — Robustezza API

- aggiungere test Ruby per la validazione dei codici di errore FFI;
- verificare aggiornamento dinamico del testo e callback di submit;
- controllare gestione shutdown e terminale.

### Fase 3 — UX TUI

- migliorare il feedback visivo della scrollbar e del focus;
- migliorare focus e feedback visivo;
- aggiungere test per resize, scrollbar e mouse scrolling.

### Fase 4 — Qualità e distribuzione

- documentare compatibilità macOS/Linux;
- automatizzare test e build;
- ridurre le modifiche non pertinenti prima del commit.
