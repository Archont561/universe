Feature: Turning a manifest into a build order

  A manifest states which packages exist, which platforms each is built for, and
  which of them must be built before the others. The planner reads only that
  statement, so the order it produces is the same on every machine and an
  unrelated edit higher in the file cannot reshuffle a build matrix.

  Scenario: Two platforms are built for every package
    Given a project whose two packages are built for linux and osx
    When the build order is planned
    Then each package is scheduled with both platforms
    And the plan is a single level because nothing depends on anything

  Scenario: A package that must be built first comes in an earlier level
    Given a project whose consumer package depends on a library package
    When the build order is planned
    Then the library is in the first level
    And the consumer is in the second level

  Scenario: A dependency on a package the manifest does not declare is refused
    Given a project whose consumer package depends on a package that does not exist
    When the manifest is read
    Then the manifest is refused because the dependency is unknown

  Scenario: A dependency cycle is refused and the package on it is named
    Given a project whose two packages depend on each other
    When the manifest is read
    Then the manifest is refused because the edges form a cycle

  Scenario: Planning the same manifest twice produces the same order
    Given a project whose two packages are built for linux and osx
    When the build order is planned
    And the build order is planned again
    Then both plans are identical
