use dashmap::DashMap;
use serde::Serialize;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;

#[derive(Serialize)]
pub struct Decision {
    pub id: String,
    pub date_mise_jour: Option<String>,
    pub code_juridiction: Option<String>,
    pub numero_dossier: Option<String>,
    pub code_publication: Option<String>,
    pub nom_juridiction: Option<String>,
    pub type_decision: Option<String>,
    pub date_lecture: Option<String>,
    pub solution: Option<String>,
    pub solution_normalisee: Option<String>,
    pub type_recours: Option<String>,
    pub numero_ecli: Option<String>,
    pub avocat_requerant: Option<String>,
    pub formation_jugement: Option<String>,
    pub date_audience: Option<String>,
    pub numero_role: Option<String>,
    pub texte_integral: Option<String>,
}

// static DEBUG_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static COMPTEURS_MANQUANTS: LazyLock<DashMap<&'static str, AtomicUsize>> =
    LazyLock::new(|| DashMap::new());
pub fn init_compteurs() {
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
        COMPTEURS_MANQUANTS.insert(key, AtomicUsize::new(0));
    }
}
