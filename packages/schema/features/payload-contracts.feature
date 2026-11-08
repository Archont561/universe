Feature: The payload contracts the portal hydrates

  The portal renders whatever the producer wrote, so the payload shape is a contract between
  two packages that never import each other's code. These scenarios hold the written contract
  against the values the workspace actually declares.

  Scenario: The published version matches the package manifest
    Given the schema package is imported
    When the declared version is compared with the package manifest
    Then they are the same version

  Scenario: Every subdir the workspace declares is a platform the payload describes
    Given the schema package is imported
    When the default Conda subdirs are read
    Then every one of them is a platform the payload names

  Scenario: A catalog payload carries everything a package card needs
    Given a produced catalog payload
    When its package card is read
    Then the card names the package, its latest version and whether its audit passed
    And the catalog names the channel a consumer pastes into their manifest
