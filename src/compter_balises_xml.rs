use quick_xml::Reader;
use quick_xml::events::Event;
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
const LIMITE_FICHIERS_DEFAUT: usize = 1000000;
fn normaliser_juridiction(code: &str) -> String {
    code.chars().take_while(|c| c.is_alphabetic()).collect()
}
fn construire_chemin(juridiction: &str, pile: &[String]) -> String {
    if juridiction.is_empty() {
        pile.join("/")
    } else {
        format!("{}/{}", juridiction, pile.join("/"))
    }
}
fn extraire_balises(path: &Path) -> HashMap<String, usize> {
    let mut compteur = HashMap::new();
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return compteur,
    };
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut pile: Vec<String> = Vec::new();
    let mut juridiction = String::new();
    let mut lire_juridiction = false;
    let mut dans_texte_integral = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let nom = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if nom == "Texte_Integral" {
                    dans_texte_integral = true;
                }
                if nom == "Code_Juridiction" {
                    lire_juridiction = true;
                }
                pile.push(nom.clone());
                if !dans_texte_integral {
                    let chemin = construire_chemin(&juridiction, &pile);
                    *compteur.entry(chemin).or_insert(0) += 1;
                }
            }
            Ok(Event::End(e)) => {
                let nom = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if nom == "Texte_Integral" {
                    dans_texte_integral = false;
                }
                if nom == "Code_Juridiction" {
                    lire_juridiction = false;
                }
                pile.pop();
            }
            Ok(Event::Text(e)) => {
                if lire_juridiction {
                    let texte: Cow<str> = e.decode().unwrap_or(Cow::Borrowed(""));
                    juridiction = normaliser_juridiction(&texte);
                }
            }
            Ok(Event::Empty(e)) => {
                if !dans_texte_integral {
                    let nom = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let mut pile_temp = pile.clone();
                    pile_temp.push(nom);
                    let chemin = construire_chemin(&juridiction, &pile_temp);
                    *compteur.entry(chemin).or_insert(0) += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    compteur
}
fn fusion(mut a: HashMap<String, usize>, b: HashMap<String, usize>) -> HashMap<String, usize> {
    for (k, v) in b {
        *a.entry(k).or_insert(0) += v;
    }
    a
}
fn parser_arguments() -> (usize, Vec<String>) {
    let mut limite = LIMITE_FICHIERS_DEFAUT;
    let mut dossiers = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--limite-fichiers" {
            if let Some(valeur) = args.next() {
                limite = valeur.parse().unwrap_or(LIMITE_FICHIERS_DEFAUT);
            }
        } else {
            dossiers.push(arg);
        }
    }
    (limite, dossiers)
}
fn main() {
    let (limite, dossiers) = parser_arguments();
    if dossiers.is_empty() {
        println!("Usage: compter_balises_xml DOSSIER");
        return;
    }
    let fichiers: Vec<PathBuf> = dossiers
        .iter()
        .flat_map(|d| WalkDir::new(d))
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().map(|e| e == "xml").unwrap_or(false))
        .take(limite)
        .collect();
    let compteur = fichiers
        .par_iter()
        .map(|p| extraire_balises(p))
        .reduce(HashMap::new, fusion);
    println!("Fichiers traités: {}", fichiers.len());
    let mut v: Vec<_> = compteur.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, c) in v {
        println!("{:<80} {}", k, c);
    }
}
