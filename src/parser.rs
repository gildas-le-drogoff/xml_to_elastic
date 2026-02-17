use crate::modele::{COMPTEURS_MANQUANTS, Decision};
use crate::texte::{
    est_chemin_autorise, normaliser_date, normaliser_espaces, normaliser_solution,
    supprimer_balises_html,
};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::Ordering;

pub fn incrementer_compteur_si_absent(val: &Option<String>, champ: &'static str) {
    if val.is_none() {
        COMPTEURS_MANQUANTS
            .get(champ)
            .unwrap()
            .fetch_add(1, Ordering::Relaxed);
    }
}

pub fn lire_decision_xml(path: &std::path::Path) -> Option<Decision> {
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
                if !est_chemin_autorise(&chemin) {
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
    incrementer_compteur_si_absent(&date_mise_jour, "date_mise_jour");
    incrementer_compteur_si_absent(&code_juridiction, "code_juridiction");
    incrementer_compteur_si_absent(&numero_dossier, "numero_dossier");
    incrementer_compteur_si_absent(&code_publication, "code_publication");
    incrementer_compteur_si_absent(&nom_juridiction, "nom_juridiction");
    incrementer_compteur_si_absent(&type_decision, "type_decision");
    incrementer_compteur_si_absent(&type_decision, "texte_integral");
    incrementer_compteur_si_absent(&date_lecture, "date_lecture");
    incrementer_compteur_si_absent(&solution, "solution");
    incrementer_compteur_si_absent(&type_recours, "type_recours");
    incrementer_compteur_si_absent(&numero_ecli, "numero_ecli");
    incrementer_compteur_si_absent(&avocat_requerant, "avocat_requerant");
    incrementer_compteur_si_absent(&formation_jugement, "formation_jugement");
    incrementer_compteur_si_absent(&date_audience, "date_audience");
    incrementer_compteur_si_absent(&numero_role, "numero_role");
    let id = id?;
    let solution_normalisee = solution.as_ref().map(|s| normaliser_solution(s));
    let texte_integral = if texte.is_empty() {
        incrementer_compteur_si_absent(&None, "texte_integral");
        None
    } else {
        Some(normaliser_espaces(&supprimer_balises_html(&texte)))
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
        date_mise_jour: date_mise_jour.as_deref().and_then(normaliser_date),
        code_juridiction,
        numero_dossier,
        code_publication,
        nom_juridiction,
        type_decision,
        date_lecture: date_lecture.as_deref().and_then(normaliser_date),
        solution,
        solution_normalisee,
        type_recours,
        numero_ecli,
        avocat_requerant,
        formation_jugement,
        date_audience: date_audience.as_deref().and_then(normaliser_date),
        numero_role,
        texte_integral,
    })
}
