-- FSV Node — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::fsv_node_sequence_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, fsv, parent_node, sequence FROM $doc_id)[0];

    LET $existing = (
        SELECT name FROM tabFSV_Node
        WHERE fsv         = $doc.fsv
          AND parent_node = $doc.parent_node
          AND sequence    = $doc.sequence
          AND name       != $doc.name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Sequence ", <string>$doc.sequence, " is already used by node '", <string>$existing.name, "' under the same parent") };
    };

    RETURN { ok: true };
};
