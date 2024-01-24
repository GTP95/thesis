#! /bin/python

import argparse
from subprocess import run

def arg_parser():
    parser = argparse.ArgumentParser(
        prog='add_data',
        description='Automatizes the process of adding some data to PEP for a given participand. Only for demo proposes'
    )
    parser.add_argument('--oauth-token-secret', type=str, help='Path to OAuth token secret JSON file', required=True)
    parser.add_argument('--pepid', type=str, help='PEP identifier of the participant', required=True)
    return parser.parse_args()

if __name__ == '__main__':
    arguments = arg_parser()

    # Add some data to the participant
    pepcli_invocation = ['pepcli', '--oauth-token-secret', arguments.oauth_token_secret, '--oauth-token-group', 'Data Administrator', 'store', '-c', 'Visit1.CSF', '-p', arguments.pepid, '-i', '~/Downloads/dog1.jpg']