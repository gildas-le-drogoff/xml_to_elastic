use html_escape::decode_html_entities;
use lazy_static::lazy_static;
use quick_xml::Reader;
use quick_xml::events::Event;
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use walkdir::WalkDir;

#[derive(Serialize)]
struct Decision {
    id: String,
    juridiction: Option<String>,
    numero_dossier: Option<String>,
    date_lecture: Option<String>,
    solution: Option<String>,
    solution_normalisee: Option<String>,
    type_recours: Option<String>,
    texte: Option<String>,
}
lazy_static! {
    static ref RE_HTML: Regex = Regex::new(r"<[^>]+>").unwrap();
    static ref RE_ESPACE: Regex = Regex::new(r"[ \t]+").unwrap();
    static ref RE_LIGNE: Regex = Regex::new(r"\n+").unwrap();
}
pub fn normaliser_solution(solution: &str) -> String {
    let s = solution.trim().to_lowercase();
    // =====================
    // Issues juridictionnelles principales
    // =====================
    if s.starts_with("rejet") || s.contains(" - rejet") {
        return "Rejet".to_string();
    }
    if s.starts_with("satisfaction totale") || s.contains("série identique - satisfaction totale")
    {
        return "Satisfaction totale".to_string();
    }
    if s.starts_with("satisfaction partielle")
        || s.contains("série identique - satisfaction partielle")
    {
        return "Satisfaction partielle".to_string();
    }
    if s.starts_with("désistement") {
        return "Désistement".to_string();
    }
    if s.starts_with("non-lieu") {
        return "Non-lieu".to_string();
    }
    // =====================
    // Procédure / Instruction
    // =====================
    if s.starts_with("expertise") || s.contains("médiation") {
        return "Mesure d'instruction".to_string();
    }
    if s.starts_with("radiation") {
        return "Radiation".to_string();
    }
    if s.starts_with("supplément d'instruction") {
        return "Mesure d'instruction".to_string();
    }
    if s.starts_with("sursis") {
        return "Sursis".to_string();
    }
    if s.starts_with("dessaisissement") {
        return "Dessaisissement".to_string();
    }
    if s.starts_with("transaction") {
        return "Transaction".to_string();
    }
    if s.starts_with("extension") {
        return "Extension".to_string();
    }
    // =====================
    // Renvois
    // =====================
    if s.starts_with("renvoi") {
        return "Renvoi".to_string();
    }
    // =====================
    // QPC
    // =====================
    if s.starts_with("qpc") {
        return "QPC".to_string();
    }
    // =====================
    // Questions juridiques spécifiques
    // =====================
    if s.starts_with("question préjudicielle") {
        return "Question préjudicielle".to_string();
    }
    if s.starts_with("demande d'avis") {
        return "Demande d'avis".to_string();
    }
    // =====================
    // Autres juridictions
    // =====================
    if s.starts_with("autres juridictions") {
        return "Autre juridiction".to_string();
    }
    // =====================
    // Cas inconnu : retourner original
    // =====================
    solution.to_string()
}
fn nettoyer_html(texte: &str) -> String {
    let sans_tags = RE_HTML.replace_all(texte, "");
    decode_html_entities(&sans_tags).to_string()
}
fn normaliser_espaces(texte: &str) -> String {
    let t = RE_ESPACE.replace_all(texte, " ");
    let t = RE_LIGNE.replace_all(&t, "\n");
    t.trim().to_string()
}
fn parser_xml(path: &Path) -> Option<Decision> {
    let file = File::open(path).ok()?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current = String::new();
    let (mut id, mut juri, mut num, mut date, mut sol, mut type_r) =
        (None, None, None, None, None, None);
    let mut texte_buffer = String::new();
    let mut dans_texte = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if current == "Texte_Integral" {
                    dans_texte = true;
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"Texte_Integral" {
                    dans_texte = false;
                }
            }
            Ok(Event::Text(e)) => {
                let val = e.decode().ok()?.to_string();
                if dans_texte {
                    texte_buffer.push_str(&val);
                    texte_buffer.push('\n');
                }
                match current.as_str() {
                    "Identification" => id = Some(val),
                    "Code_Juridiction" => juri = Some(val),
                    "Numero_Dossier" => num = Some(val),
                    "Date_Lecture" => date = Some(val),
                    "Solution" => sol = Some(val),
                    "Type_Recours" => type_r = Some(val),
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    let id = id?.replace(".xml", "");
    let solution_normalisee = sol.as_ref().map(|s| normaliser_solution(s));
    let texte = if texte_buffer.is_empty() {
        None
    } else {
        Some(normaliser_espaces(&nettoyer_html(&texte_buffer)))
    };
    Some(Decision {
        id,
        juridiction: juri,
        numero_dossier: num,
        date_lecture: date,
        solution: sol,
        solution_normalisee,
        type_recours: type_r,
        texte,
    })
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: xml_to_elastic DOSSIER OUTPUT");
        return;
    }
    let dossier = &args[1];
    let output_path = args[2].clone();
    let paths: Vec<PathBuf> = WalkDir::new(dossier)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("xml"))
        .map(|e| e.into_path())
        .collect();
    let (tx, rx) = channel::<Decision>();
    let writer_thread = std::thread::spawn(move || {
        let mut output = File::create(output_path).expect("Erreur création fichier");
        let mut count = 0;
        for decision in rx {
            let routing = decision.juridiction.as_deref().unwrap_or("unknown");
            let meta = format!(
                r#"{{"index":{{"_index":"decisions","_id":"{}","routing":"{}"}}}}"#,
                decision.id, routing
            );
            if let Ok(json) = serde_json::to_string(&decision) {
                writeln!(output, "{}", meta).expect("Erreur écriture meta");
                writeln!(output, "{}", json).expect("Erreur écriture json");
                count += 1;
            }
        }
        count
    });
    paths.into_par_iter().for_each_with(tx, |tx, path| {
        if let Some(decision) = parser_xml(&path) {
            let _ = tx.send(decision);
        }
    });
    let final_count = writer_thread.join().unwrap();
    println!("Total traité : {}", final_count);
}
