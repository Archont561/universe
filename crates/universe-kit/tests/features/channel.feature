Feature: Publishing a Conda channel to a branch and restoring it with Git

  A channel is a directory of packages and the metadata a solver reads to find them.
  When that directory lives on a Git branch, the consumer needs nothing but Git: the
  restore is a clone, and the digests in the metadata are what make the clone
  trustworthy. These scenarios follow that loop from the publisher's machine to a
  consumer's empty directory.

  Scenario: An audited package is published to the branch and restored with Git alone
    Given an audited package with a channel branch store
    When the channel is published
    Then the branch holds the channel metadata and the package at the standard paths
    When a consumer clones the channel branch
    Then the clone's package matches the digest the fragment recorded

  Scenario: Publishing the same artifacts twice pushes no second commit
    Given an audited package with a channel branch store
    When the channel is published
    And the channel is published again
    Then the branch holds exactly one commit

  Scenario: An artifact that no longer matches its fragment stops the publish
    Given an audited package with a channel branch store
    And the artifact is altered after the audit
    When the channel is published
    Then the publish names the artifact it cannot vouch for
    And the branch does not exist

  Scenario: A store that refuses snapshots above four bytes publishes nothing
    Given a channel whose store refuses snapshots above four bytes
    When the channel is published
    Then the publish is refused before anything is pushed
    And the branch does not exist

  Scenario: A branch that no longer matches its fragments is reported as drift
    Given an audited package with a channel branch store
    When the channel is published
    And the package is rebuilt and its fragment updated
    And the channel is checked
    Then the check reports drift against the fragments
