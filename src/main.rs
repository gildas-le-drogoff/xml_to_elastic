use crate::modele::{COMPTEURS_MANQUANTS, Decision, init_compteurs};
use crate::parser::lire_decision_xml;
use crate::texte::extraire_nom_index;

use owo_colors::OwoColorize;
use rayon::prelude::*;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use walkdir::WalkDir;

mod modele;
mod parser;
mod texte;

fn main() {
    init_compteurs();

    let arguments: Vec<String> = env::args().collect();

    if arguments.len() < 3 {
        eprintln!(
            "{} {} <dossier1> [dossier2 ... dossierN] <output.jsonl>",
            "usage:".red().bold(),
            arguments[0].yellow()
        );
        std::process::exit(1);
    }

    let chemin_sortie = PathBuf::from(&arguments[arguments.len() - 1]);

    if chemin_sortie.exists() && chemin_sortie.is_dir() {
        eprintln!(
            "{} {}",
            "Erreur:".red().bold(),
            "le dernier argument doit être un fichier, pas un dossier".red()
        );
        std::process::exit(1);
    }

    let dossiers = &arguments[1..arguments.len() - 1];

    println!(
        "{} {}",
        "Dossiers analysés:".cyan().bold(),
        dossiers.len().to_string().yellow()
    );

    let mut chemins_xml: Vec<PathBuf> = Vec::with_capacity(1_000_000);

    for dossier in dossiers {
        println!("{} {}", "Scan:".blue().bold(), dossier.bright_blue());

        let iterateur = WalkDir::new(dossier)
            .into_iter()
            .filter_map(|entree| entree.ok())
            .filter(|entree| {
                entree.file_type().is_file()
                    && entree
                        .path()
                        .extension()
                        .map(|ext| ext == "xml")
                        .unwrap_or(false)
            })
            .map(|entree| entree.into_path());

        chemins_xml.extend(iterateur);
    }

    println!(
        "{} {}",
        "Fichiers XML trouvés:".green().bold(),
        chemins_xml.len().to_string().bright_green().bold()
    );

    use crossbeam_channel::unbounded;

    let (tx, rx) = unbounded::<Decision>();

    let thread_ecriture = std::thread::spawn({
        let chemin_sortie = chemin_sortie.clone();

        move || {
            let mut fichier = File::create(chemin_sortie).unwrap();

            let mut total_ecrit = 0usize;

            for decision in rx {
                let nom_index = extraire_nom_index(&*decision.id);

                let meta = format!(
                    r#"{{"index":{{"_index":"{}","_id":"{}"}}}}"#,
                    nom_index, decision.id
                );

                writeln!(fichier, "{}", meta).unwrap();

                writeln!(fichier, "{}", serde_json::to_string(&decision).unwrap()).unwrap();

                total_ecrit += 1;

                if total_ecrit % 100_000 == 0 {
                    println!(
                        "{} {}",
                        "Progression:".magenta().bold(),
                        total_ecrit.to_string().bright_magenta()
                    );
                }
            }

            total_ecrit
        }
    });

    println!("{}", "Traitement parallèle en cours...".cyan().bold());

    chemins_xml
        .into_par_iter()
        .for_each_with(tx.clone(), |tx, chemin| {
            if let Some(decision) = lire_decision_xml(&chemin) {
                let _ = tx.send(decision);
            }
        });

    drop(tx);

    let total = thread_ecriture.join().unwrap();

    println!(
        "{} {}",
        "Total traité:".green().bold(),
        total.to_string().bright_green().bold()
    );

    println!("\n{}", "Statistiques des champs manquants".yellow().bold());

    let mut stats: Vec<(&str, usize)> = COMPTEURS_MANQUANTS
        .iter()
        .map(|e| (*e.key(), e.value().load(Ordering::Relaxed)))
        .collect();

    stats.sort_by(|a, b| b.1.cmp(&a.1));

    for (champ, nombre) in stats {
        let pourcentage = if total > 0 {
            (nombre as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let couleur = if pourcentage > 50.0 {
            format!("{:.1}%", pourcentage).red().to_string()
        } else if pourcentage > 20.0 {
            format!("{:.1}%", pourcentage).yellow().to_string()
        } else {
            format!("{:.1}%", pourcentage).green().to_string()
        };

        println!(
            "{} {} {}",
            champ.bright_white().bold(),
            nombre.to_string().bright_blue(),
            couleur
        );
    }

    println!("{}", "Terminé.".bright_green().bold());
}
