import argparse
from subprocess import run
from debugprint import Debug
import re


def arg_parser():
    """Parse the command line argument --oauth-token-secret and the participant's IRMA credential."""
    parser = argparse.ArgumentParser(
        prog='enroll',
        description='Automatizes the process of creating a new participant and giving him access to a column group'
    )
    parser.add_argument('--oauth-token-secret', type=str, help='Path to OAuth token secret JSON file')
    parser.add_argument('--attribute', type=str, help='Participant\'s IRMA credential')
    parser.add_argument('--column-group', type=str, help='Column group to which the participant will be given access',
                        required=False, default='Visits')
    parser.add_argument('--date-of-birth', type=str, help='Participant\'s date of birth')
    parser.add_argument('--first-name', type=str, help='Participant\'s first name')
    parser.add_argument('--last-name', type=str, help='Participant\'s last name')
    parser.add_argument('--middle-name', type=str, help='Participant\'s middle name', required=False, default='')
    args = parser.parse_args()
    return args


if __name__ == '__main__':
    debug = Debug("enroll")
    arguments = arg_parser()

    # Create a new group
    pepcli_invocation = 'pepcli --oauth-token-secret ' + arguments.oauth_token_secret + ' --oauth-token-group "Access Administrator" asa group create ' + arguments.attribute
    debug(pepcli_invocation)
    run(pepcli_invocation)

    # Add a new member to the group
    pepcli_invocation = 'pepcli --oauth-token-secret ' + arguments.oauth_token_secret + ' --oauth-token-group "Access Administrator" asa user addTo ' + arguments.attribute + ' ' + arguments.attribute
    debug(pepcli_invocation)
    run(pepcli_invocation)

    # Create a new participant group
    pepcli_invocation = 'pepcli --oauth-token-secret ' + arguments.oauth_token_secret + ' --oauth-token-group "Data Administrator" ama group create ' + arguments.attribute
    debug(pepcli_invocation)
    run(pepcli_invocation)

    # Create participant group access rules giving the user access to the new participant group
    pepcli_invocation = 'pepcli --oauth-token-secret ' + arguments.oauth_token_secret + ' --oauth-token-group "Access Administrator" ama pgar create ' + arguments.attribute + ' ' + arguments.attribute + ' ' + 'access'
    debug(pepcli_invocation)
    run(pepcli_invocation)
    pepcli_invocation = 'pepcli --oauth-token-secret ' + arguments.oauth_token_secret + ' --oauth-token-group "Access Administrator" ama pgar create ' + arguments.attribute + ' ' + arguments.attribute + ' ' + 'enumerate'
    debug(pepcli_invocation)
    run(pepcli_invocation)

    # Create a column access rule giving the user read-only access to the Visits column group
    pepcli_invocation = 'pepcli --oauth-token-secret OAuthTokenSecret.json --oauth-token-group "Access Administrator" ama cgar create ' + arguments.column_group + arguments.attribute + 'read'
    debug(pepcli_invocation)
    run(pepcli_invocation)

    # Crete new participant
    pepcli_invocation = 'pepcli --oauth-token-secret OAuthTokenSecret.json --oauth-token-group "Data Administrator" register participant --force --fist-name ' + arguments.first_name + ' --last-name ' + arguments.last_name + ' --middle-name ' + arguments.middle_name + ' --date-of-birth ' + arguments.date_of_birth
    debug(pepcli_invocation)
    result = run(pepcli_invocation, capture_output=True)
    output = result.stdout.decode('utf-8')
    debug(output)

    # Extract the participant's identifier from the output
    pattern = re.compile(r"completed enrollment! Generated participant with identifier: [A-Za-z0-9]+", re.IGNORECASE)
    identifier = pattern.match(output)
    debug('extracted identifier: ' + identifier)

    # Add participant to the previously created participant group
    pepcli_invocation = 'pepcli --oauth-token-secret OAuthTokenSecret.json --oauth-token-group "Data Administrator" ama group addTo ' + arguments.attribute + identifier
    debug(pepcli_invocation)
    run(pepcli_invocation)
