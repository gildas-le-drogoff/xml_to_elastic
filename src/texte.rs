use html_escape::decode_html_entities;
use std::collections::HashSet;

use chrono::NaiveDate;
use lazy_static::lazy_static;
use regex::Regex;

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
pub fn normaliser_date(date: &str) -> Option<String> {
    let formats = ["%Y-%m-%d", "%d-%m-%Y", "%d/%m/%Y", "%Y%m%d"];
    for fmt in formats {
        if let Ok(d) = NaiveDate::parse_from_str(date, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    eprintln!("Date invalide: {}", date);
    None
}

pub fn est_chemin_autorise(chemin: &str) -> bool {
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
pub fn supprimer_balises_html(texte: &str) -> String {
    let sans_tags = RE_HTML.replace_all(texte, "");
    decode_html_entities(&sans_tags).to_string()
}
pub fn normaliser_espaces(texte: &str) -> String {
    let t = RE_ESPACE.replace_all(texte, " ");
    let t = RE_LIGNE.replace_all(&t, "\n");
    t.trim().to_string()
}
pub fn extraire_nom_index(id: &str) -> String {
    let prefix = id.split('_').next().unwrap_or("");
    let juridiction = match prefix {
        "ORTA" | "DTA" => "ta",
        "ORCA" | "DCA" => "caa",
        "ORCE" | "DCE" => "ce",
        _ => "inconnu",
    };
    format!("{}_decisions", juridiction)
}
