# xml_to_elastic

## Objet

`xml_to_elastic` convertit les fichiers XML de jurisprudence administrative française (TA, CAA, CE) en fichier Bulk JSON directement indexable dans Elasticsearch.

Sources compatibles :

- Open Data Conseil d’État
- ArianeWeb
- Judilibre
- exports juridictionnels standards

Conçu pour traiter plusieurs millions de décisions.

## Juridictions supportées

- TA — Tribunaux administratifs
- CAA — Cours administratives d’appel
- CE — Conseil d’État

Le champ XML utilisé :

```
Code_Juridiction
```

Utilisé dans Elasticsearch comme :

- champ `juridiction`
- routing

## Format XML attendu

Exemple :

```xml
<Decision>
  <Identification>CE_123456.xml</Identification>
  <Code_Juridiction>CE</Code_Juridiction>
  <Numero_Dossier>123456</Numero_Dossier>
  <Date_Lecture>2023-01-01</Date_Lecture>
  <Solution>Rejet</Solution>
  <Type_Recours>Excès de pouvoir</Type_Recours>
  <Texte_Integral>
    <![CDATA[
<p>Texte de la décision...</p>
]]>
  </Texte_Integral>
</Decision>
```

## Document Elasticsearch produit

```json
{
  "id": "CE_123456",
  "juridiction": "CE",
  "numero_dossier": "123456",
  "date_lecture": "2023-01-01",
  "solution": "Rejet",
  "solution_normalisee": "Rejet",
  "type_recours": "Excès de pouvoir",
  "texte": "Texte nettoyé"
}
```

## Format Bulk généré

```
{"index":{"_index":"decisions","_id":"CE_123456","routing":"CE"}}
{document}
```

Compatible directement avec l’API Bulk.

## Fonctionnement

Traitement :

```
XML
 ↓
Parsing
 ↓
Nettoyage HTML
 ↓
Normalisation
 ↓
Bulk JSON
```

Parallélisé avec Rayon.

## Compilation

Cargo.toml :

```toml
[package]
name = "xml_to_elastic"
version = "0.1.0"
edition = "2024"

[dependencies]
crossbeam-channel = "0.5"
html-escape = "0.2"
lazy_static = "1.5"
quick-xml = "0.39"
rayon = "1.11"
regex = "1.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
walkdir = "2.5"
```

Build :

```bash
cargo build --release
```

Binaire :

```
target/release/xml_to_elastic
```

## Utilisation

Syntaxe :

```bash
xml_to_elastic DOSSIER_XML OUTPUT_BULK
```

Exemple :

```bash
xml_to_elastic TA bulk.json
```

## Indexation Elasticsearch

```bash
curl -X POST localhost:9200/_bulk \
-H "Content-Type: application/x-ndjson" \
--data-binary "@bulk.json"
```

## Mapping recommandé

```json
{
  "settings": {
    "number_of_shards": 6,
    "number_of_replicas": 0
  },
  "mappings": {
    "properties": {
      "id": { "type": "keyword" },
      "juridiction": { "type": "keyword" },
      "numero_dossier": { "type": "keyword" },
      "date_lecture": { "type": "date" },
      "solution": { "type": "keyword" },
      "solution_normalisee": { "type": "keyword" },
      "type_recours": { "type": "keyword" },
      "texte": {
        "type": "text",
        "analyzer": "french"
      }
    }
  }
}
```

## Volumes supportés

| Juridiction | Fichiers    | Taille     |
| ----------- | ----------- | ---------- |
| TA          | 773 969     | 8,9 G      |
| CAA         | 108 595     | 1,7 G      |
| CE          | 34 985      | 338 M      |
| **TOTAL**   | **917 549** | **10,9 G** |

Total : plusieurs millions.

## Performance

Mesure réelle sur corpus TA :

Corpus :

- 773 969 décisions XML
- 8,9 Go

Commande :

```bash
xml_to_elastic TA bulk_all.json
```

Résultat :

- Temps réel : 85,11 secondes
- Débit : ~9 095 décisions / seconde
- Débit : ~545 000 décisions / minute
- Débit : ~32,7 millions décisions / heure

Utilisation ressources :

- CPU : 219 % (≈ 2,2 cœurs utilisés)
- Mémoire : < 1 Mo RSS
- Limitation principale : lecture disque

Estimation corpus complet actuel :

- 917 549 décisions
- ~10,9 Go

Temps total estimé :

- ~80 secondes

Le traitement est :

- parallèle
- streaming
- sans accumulation mémoire

Le facteur limitant est le débit de lecture du système de fichiers, pas le CPU ni la RAM.

## Résultat

Index Elasticsearch :

```
decisions
```

Contenant l’ensemble des décisions TA, CAA, CE.

Utilisable pour :

- recherche plein texte
- analyse juridique
- statistiques
- NLP
- RAG
