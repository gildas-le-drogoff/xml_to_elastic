use dashmap::DashMap;
use html_escape::decode_html_entities;
use lazy_static::lazy_static;
use quick_xml::Reader;
use quick_xml::events::Event;
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;
#[derive(Serialize)]
struct Decision {
    id: String,
    date_mise_jour: Option<String>,
    code_juridiction: Option<String>,
    numero_dossier: Option<String>,
    code_publication: Option<String>,
    nom_juridiction: Option<String>,
    type_decision: Option<String>,
    date_lecture: Option<String>,
    solution: Option<String>,
    solution_normalisee: Option<String>,
    type_recours: Option<String>,
    numero_ecli: Option<String>,
    avocat_requerant: Option<String>,
    formation_jugement: Option<String>,
    date_audience: Option<String>,
    numero_role: Option<String>,
    texte_integral: Option<String>,
}
// static DEBUG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MISSING_COUNTERS: LazyLock<DashMap<&'static str, AtomicUsize>> =
    LazyLock::new(|| DashMap::new());
fn init_counters() {
    for key in [
        "date_mise_jour",
        "code_juridiction",
        "numero_dossier",
        "code_publication",
        "nom_juridiction",
        "type_decision",
        "date_lecture",
        "solution",
        "type_recours",
        "numero_ecli",
        "avocat_requerant",
        "formation_jugement",
        "date_audience",
        "numero_role",
        "texte_integral",
    ] {
        MISSING_COUNTERS.insert(key, AtomicUsize::new(0));
    }
}
fn inc_if_absent(val: &Option<String>, champ: &'static str) {
    if val.is_none() {
        MISSING_COUNTERS
            .get(champ)
            .unwrap()
            .fetch_add(1, Ordering::Relaxed);
    }
}
lazy_static! {
    static ref RE_HTML: Regex = Regex::new(r"<[^>]+>").unwrap();
    static ref RE_ESPACE: Regex = Regex::new(r"[ \t]+").unwrap();
    static ref RE_LIGNE: Regex = Regex::new(r"\n+").unwrap();
    static ref WHITELIST: HashSet<&'static str> = HashSet::from([
        "Document/Donnees_Techniques/Identification",
        "Document/Donnees_Techniques/Date_Mise_Jour",
        "Document/Dossier/Code_Juridiction",
        "Document/Dossier/Numero_Dossier",
        "Document/Dossier/Code_Publication",
        "Document/Dossier/Nom_Juridiction",
        "Document/Dossier/Type_Decision",
        "Document/Dossier/Date_Lecture",
        "Document/Dossier/Solution",
        "Document/Dossier/Type_Recours",
        "Document/Dossier/Numero_ECLI",
        "Document/Dossier/Avocat_Requerant",
        "Document/Audience/Formation_Jugement",
        "Document/Audience/Date_Audience",
        "Document/Audience/Numero_Role",
        "Document/Decision/Texte_Integral",
    ]);
}
fn autorise(chemin: &str) -> bool {
    WHITELIST.contains(chemin)
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
fn parser_xml(path: &std::path::Path) -> Option<Decision> {
    let file = File::open(path).ok()?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut pile = Vec::new();
    let mut texte = String::new();
    let mut dans_texte = false;
    let mut id = None;
    let mut date_mise_jour = None;
    let mut code_juridiction = None;
    let mut numero_dossier = None;
    let mut code_publication = None;
    let mut nom_juridiction = None;
    let mut type_decision = None;
    let mut date_lecture = None;
    let mut solution = None;
    let mut type_recours = None;
    let mut numero_ecli = None;
    let mut avocat_requerant = None;
    let mut formation_jugement = None;
    let mut date_audience = None;
    let mut numero_role = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                pile.push(tag.clone());
                if pile.join("/") == "Document/Decision/Texte_Integral" {
                    dans_texte = true;
                }
            }
            Ok(Event::End(_)) => {
                if pile.join("/") == "Document/Decision/Texte_Integral" {
                    dans_texte = false;
                }
                pile.pop();
            }
            Ok(Event::Text(e)) => {
                let chemin = pile.join("/");
                let val = e.decode().ok()?.to_string();
                if dans_texte {
                    texte.push_str(&val);
                    texte.push('\n');
                }
                if !autorise(&chemin) {
                    buf.clear();
                    continue;
                }
                match chemin.as_str() {
                    "Document/Donnees_Techniques/Identification" => id = Some(val),
                    "Document/Donnees_Techniques/Date_Mise_Jour" => date_mise_jour = Some(val),
                    "Document/Dossier/Code_Juridiction" => code_juridiction = Some(val),
                    "Document/Dossier/Numero_Dossier" => numero_dossier = Some(val),
                    "Document/Dossier/Code_Publication" => code_publication = Some(val),
                    "Document/Dossier/Nom_Juridiction" => nom_juridiction = Some(val),
                    "Document/Dossier/Type_Decision" => type_decision = Some(val),
                    "Document/Dossier/Date_Lecture" => date_lecture = Some(val),
                    "Document/Dossier/Solution" => solution = Some(val),
                    "Document/Dossier/Type_Recours" => type_recours = Some(val),
                    "Document/Dossier/Numero_ECLI" => numero_ecli = Some(val),
                    "Document/Dossier/Avocat_Requerant" => avocat_requerant = Some(val),
                    "Document/Audience/Formation_Jugement" => formation_jugement = Some(val),
                    "Document/Audience/Date_Audience" => date_audience = Some(val),
                    "Document/Audience/Numero_Role" => numero_role = Some(val),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    inc_if_absent(&date_mise_jour, "date_mise_jour");
    inc_if_absent(&code_juridiction, "code_juridiction");
    inc_if_absent(&numero_dossier, "numero_dossier");
    inc_if_absent(&code_publication, "code_publication");
    inc_if_absent(&nom_juridiction, "nom_juridiction");
    inc_if_absent(&type_decision, "type_decision");
    inc_if_absent(&type_decision, "texte_integral");
    inc_if_absent(&date_lecture, "date_lecture");
    inc_if_absent(&solution, "solution");
    inc_if_absent(&type_recours, "type_recours");
    inc_if_absent(&numero_ecli, "numero_ecli");
    inc_if_absent(&avocat_requerant, "avocat_requerant");
    inc_if_absent(&formation_jugement, "formation_jugement");
    inc_if_absent(&date_audience, "date_audience");
    inc_if_absent(&numero_role, "numero_role");
    let id = id?;
    let solution_normalisee = solution.as_ref().map(|s| normaliser_solution(s));
    let texte_integral = if texte.is_empty() {
        inc_if_absent(&None, "texte_integral");
        None
    } else {
        Some(normaliser_espaces(&nettoyer_html(&texte)))
    };
    // let index = DEBUG_COUNT.fetch_add(1, Ordering::Relaxed);
    // if index < 2 {
    //     println!("DEBUG {}", path.display());
    //     println!(
    //         "DEBUG {} -> texte size {:?}",
    //         path.display(),
    //         texte_integral.as_ref().map(|s| s.len())
    //     );
    // }
    Some(Decision {
        id,
        date_mise_jour,
        code_juridiction,
        numero_dossier,
        code_publication,
        nom_juridiction,
        type_decision,
        date_lecture,
        solution,
        solution_normalisee,
        type_recours,
        numero_ecli,
        avocat_requerant,
        formation_jugement,
        date_audience,
        numero_role,
        texte_integral,
    })
}
fn main() {
    init_counters();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <dossier1> [dossier2 ... dossierN] <output.jsonl>",
            args[0]
        );
        std::process::exit(1);
    }
    let output = PathBuf::from(&args[args.len() - 1]);
    if output.exists() && output.is_dir() {
        eprintln!("Erreur: le dernier argument doit être un fichier, pas un dossier");
        std::process::exit(1);
    }
    let dossiers = &args[1..args.len() - 1];
    let mut paths: Vec<PathBuf> = Vec::with_capacity(1_000_000);
    for dossier in dossiers {
        let iter = WalkDir::new(dossier)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .map(|ext| ext == "xml")
                        .unwrap_or(false)
            })
            .map(|e| e.into_path());
        paths.extend(iter);
    }
    println!("Fichiers XML trouvés: {}", paths.len());
    use crossbeam_channel::unbounded;
    let (tx, rx) = unbounded::<Decision>();
    let writer = std::thread::spawn({
        let output = output.clone();
        move || {
            let mut f = File::create(output).unwrap();
            let mut count = 0usize;
            for d in rx {
                let meta = format!(r#"{{"index":{{"_id":"{}"}}}}"#, d.id);
                writeln!(f, "{}", meta).unwrap();
                writeln!(f, "{}", serde_json::to_string(&d).unwrap()).unwrap();
                count += 1;
            }
            count
        }
    });
    paths.into_par_iter().for_each_with(tx.clone(), |tx, path| {
        if let Some(d) = parser_xml(&path) {
            let _ = tx.send(d);
        }
    });
    drop(tx);
    let total = writer.join().unwrap();
    println!("Total traité: {}", total);
    println!("\n--- Statistiques ---");
    let mut stats: Vec<(&str, usize)> = MISSING_COUNTERS
        .iter()
        .map(|e| (*e.key(), e.value().load(Ordering::Relaxed)))
        .collect();
    stats.sort_by(|a, b| b.1.cmp(&a.1));
    for (champ, count) in stats {
        let pct = if total > 0 {
            (count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!("{:25} {:>10} ({:>5.1}%)", champ, count, pct);
    }
}
