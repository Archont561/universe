Feature: A static channel that a solver can read and a republish cannot corrupt

  A channel is a tree of files, and every file in it is a pure function of the
  audit fragments it was rendered from. That is what lets a publish be judged
  instead of hoped for: two renders of the same artifacts are byte-identical, the
  numbers a solver reads are the ones the auditor measured, and a second publish
  of unchanged bytes writes nothing at all.

  Scenario: Every served subdir is answered even when it holds nothing
    Given a channel serving linux-64 and noarch
    And one audited artifact for linux-64
    When the channel is rendered
    Then it answers for noarch as well
    And it names the channel in its channel data

  Scenario: The repodata repeats the numbers the audit measured
    Given a channel serving linux-64 and noarch
    And one audited artifact for linux-64
    When the channel is rendered
    Then the linux-64 repodata names the artifact and its digest

  Scenario: Two renders of the same fragments are byte-identical
    Given a channel serving linux-64 and noarch
    And one audited artifact for linux-64
    When the channel is rendered twice
    Then both renders have the same files and the same bytes

  Scenario: A fragment for a subdir the channel does not serve is refused
    Given a channel serving linux-64 only
    And one audited artifact for noarch
    When the channel is rendered
    Then rendering is refused because the subdir is not served

  Scenario: Republishing unchanged bytes writes nothing
    Given a store that already holds one artifact
    When the same artifact is published again
    Then the publish is a no-op
    And the artifact is reported as unchanged

  Scenario: Changed bytes under an occupied name are refused
    Given a store that already holds one artifact
    When different bytes are published under the same name
    Then the publish is refused and the stored bytes are kept

  Scenario: Retention keeps the newest builds of each version
    Given a channel holding three builds of one version
    When retention keeps one build of each version
    Then two builds are pruned and the newest is kept

  Scenario: Retention that would empty the channel is refused
    Given a channel holding three builds of one version
    When retention keeps nothing
    Then retention is refused because the channel would be empty
