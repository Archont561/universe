Feature: Publishing built artifacts to a store

  A store keeps what it was first given, so the publisher has to compare digests
  before it writes. These scenarios follow one artifact from an audited build to a
  destination that owns the file's name: what lands, what a second run does, and what
  happens when the same name arrives with different bytes.

  Scenario: An artifact is published to the store under its subdir
    Given an audited package with a filesystem store
    When the artifacts are published
    Then the store holds the artifact under its subdir

  Scenario: Publishing the same build twice uploads nothing the second time
    Given an audited package with a filesystem store
    When the artifacts are published
    And the artifacts are published again
    Then the second publish reports the artifact as already there

  Scenario: A changed build under an occupied name is refused
    Given an audited package with a filesystem store
    And the published artifact is replaced by a different build
    When the artifacts are published
    Then the publish refuses the occupied name
    And the store still holds the bytes it held

  Scenario: A dry run writes nothing
    Given an audited package with a filesystem store
    When the artifacts are published as a dry run
    Then the store directory does not exist

  Scenario: A branch store cannot be uploaded to
    Given an audited package with a filesystem store and a branch store
    When the branch store is named for an upload
    Then the publish is a usage error naming the store kind
