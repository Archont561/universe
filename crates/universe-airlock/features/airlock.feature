Feature: The Airlock consumer on a host that cannot install anything

  Universe Airlock is the side of the product that runs after the network is
  gone. It reports what it is, in the two forms a script and a human each need,
  and every verb it does not implement is refused by name instead of being
  accepted and forgotten.

  Scenario: The version is reported in the form a script reads
    Given the airlock command line "version --json"
    When it is executed
    Then it succeeds with nothing on stderr
    And the report names universe-airlock and the release train version

  Scenario: The version is reported in the form a human reads
    Given the airlock command line "--version"
    When it is executed
    Then it succeeds with nothing on stderr
    And the report is the binary name and version on one line

  Scenario: The consumer lists what it can do
    Given the airlock command line "--help"
    When it is executed
    Then it succeeds with nothing on stderr
    And the help names the version command

  Scenario: A producer verb is refused by name
    Given the airlock command line "publish"
    When it is executed
    Then it exits with the usage code
    And the refusal names the argument it does not implement
