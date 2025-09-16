Feature: Transaction state machine
  As a domain implementer
  I want transactions to evolve through valid states only
  So that undesirable states are unrepresentable

  Scenario: Successful commit path
    Given Transaction is Idle
    When we Start
    And we ValidateOk
    And we Commit
    Then state is Committed
    And Expect Event Stream is empty

  Scenario: Cancellation path
    Given Transaction is Idle
    When we Start
    And we Cancel
    Then state is Cancelled
    And Expect Event Stream is empty

  Scenario: Validation failure path
    Given Transaction is Idle
    When we Start
    And we ValidateFail
    Then state is Failed
    And Expect Event Stream is empty

  Scenario: Illegal transition is rejected
    Given Transaction is Idle
    When we Commit
    Then transition is invalid
