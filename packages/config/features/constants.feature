Feature: The constants every workspace package shares

  A package that hard-codes a path or a platform name drifts from the repository one release
  at a time. These constants are the single place those values live, so the behaviour that
  matters is that they still describe *this* checkout.

  Scenario: The repository root is the directory that holds the workspace manifests
    Given the shared constants are imported
    When the repository root is asked for
    Then it holds the Pixi manifest, the Cargo manifest and the root package manifest
    And the crates and packages directories sit inside it

  Scenario: The published version matches the package manifest
    Given the version this package declares
    When it is compared with the package manifest
    Then the two are the same version

  Scenario: Every default Conda subdir is a platform name a channel can serve
    Given the subdirs a channel may serve
    When they are read
    Then every one of them is non-empty and lowercase
    And every one of them is unique
