# Utilisation avec Elasticsearch

Ce guide décrit la procédure pour générer et indexer les décisions TA, CAA, CE dans Elasticsearch.

# 1. Générer le fichier bulk

Compiler et exécuter :

```bash
cargo fmt
cargo run --release -- TA bulk_all.json
```

Résultat :

```
bulk_all.json
```

Ce fichier contient toutes les décisions au format Bulk Elasticsearch.

# 2. Découper le fichier bulk

Découper en blocs plus petits :

```bash
rm -f bulk_part_*

split -l 20000 bulk_all.json bulk_part_

ls -lh bulk_part_*
```

# 3. Vérifier le format

Vérifier que le fichier se termine par un newline :

```bash
tail -c 1 bulk_part_aa | od -An -t x1
```

Résultat attendu :

```
0a
```

# 4. Supprimer l’index existant

```bash
curl -X DELETE http://localhost:9200/decisions || true
```

# 5. Indexer dans Elasticsearch

```bash
for f in bulk_part_*
do
    echo "Indexing $f"
    curl -s \
        -H "Content-Type: application/x-ndjson" \
        -X POST localhost:9200/_bulk \
        --data-binary "@$f" |
        jq -e '.errors == false' > /dev/null

    if [ $? -ne 0 ]
    then
        echo "ERREUR dans $f"
        exit 1
    fi
    echo "OK"
done
```

# 6. Vérifier le nombre de documents

```bash
curl localhost:9200/decisions/_count?pretty
```

```bash
chmod +x ingest.sh
bash ingest.sh
```
