Feature: Which component a `universe` command line belongs to

  The facade is the only command a user types, so the rule it applies decides
  which executable runs. It owns no behaviour of its own: a producer verb goes to
  the producer, the airlock namespace goes to the consumer, and the two verbs that
  produce — rendering an entrypoint, publishing the ref it clones — stay with the
  producer even though the namespace is the consumer's.

  Scenario: A command with no namespace belongs to the producer
    Given the command line "universe pages build --offline"
    When it is routed
    Then the producer runs it
    And the arguments reach it unchanged

  Scenario: The explicit producer namespace is not forwarded
    Given the command line "universe kit pages build"
    When it is routed
    Then the producer runs it
    And the arguments reach it without the namespace

  Scenario: The airlock namespace belongs to the consumer
    Given the command line "universe airlock exec -- cargo test"
    When it is routed
    Then the consumer runs it
    And the arguments reach it without the namespace

  Scenario: Rendering an entrypoint is producer work in the consumer's namespace
    Given the command line "universe airlock render --check"
    When it is routed
    Then the producer runs it
    And the arguments reach it unchanged

  Scenario: Publishing the ref the entrypoint clones is producer work
    Given the command line "universe airlock publish-bootstrap --binary boot"
    When it is routed
    Then the producer runs it
    And the arguments reach it unchanged
