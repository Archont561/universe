Feature: What an audit learns from a Conda archive and a recipe

  An archive is the last thing a consumer receives, and a recipe is the last
  configuration before one is built. Both fail silently by default — an archive
  whose digest nobody measured, a licence file named but not carried, a source
  that downloads different bytes next week — so each is refused here, by code,
  while the fix is still cheap.

  Scenario: An archive reports the identity its index declares
    Given an archive carrying its index record
    When the archive is opened
    Then the reading names the package, version, build and subdir the index declares
    And the reading is as large as the file on disk

  Scenario: The digest is of the bytes, not of the file name
    Given an archive carrying its index record
    When the archive is opened twice
    Then both readings name the same digest
    When a byte of the archive changes
    And the archive is opened again
    Then the reading names a different digest

  Scenario: An archive with no index record is refused
    Given an archive with no index record
    When the archive is opened
    Then the reading is refused because the index is missing

  Scenario: A licence file named by the archive must be carried by it
    Given an archive whose about record names a licence file it does not carry
    When the licence is audited
    Then the audit reports one violation
    And the violation names the licence file that is missing

  Scenario: A recipe whose source is not pinned is refused
    Given a recipe whose url source names no digest
    When the recipe is linted
    Then the lint reports one finding for the missing digest
