## rqm - Remote qBittorrent Manager

rqm is a small utility which retrieves complete bittorrent downloads from a remote server running qBittorrent.  It periodically checks using the qBittorrent API 
to see if there's anything to retrieve, and if so, transfers the files using rsync and deletes them from the qBittorrent server.  It uses qBittorrent categories
to determine what directory to place the file in and what scripts to run.

### Getting started

You'll first want to set up a config file for rqm.  By default, rqm will look for a file named "rqm.conf" in the current directory, but this could be placed anywhere
via the command line.

**[Global]**

**logfile** location of logfile

**loglevel** either "error" or "info"

**[Qbittorrent]**

**url** the url of the server

**username** a username to log in

**password** an *encrypted* password.  rqm will encrypt your password from the command line using the -p switch

**check_interval** how often to poke the server when there's nothing to retrieve.  If rqm sees a file will be complete soon, it will check using the lower of this number or the file completion estimate from qBittorrent.

**[Post Processing]**

**rsync** the path to the rsync binary

**server** the server to retrieve the file from.  Usually the same server as the one the qBittorrent url points to, but this is used for rsync
rqm assumes that rsync has already been set up with credentials for passwordless transfer of files.  There's no mechanism to enter rsync passwords by design

**partialpath** where to store files in transit

**localpath** default final location after transfer

**set_owner**, **set_group** ownership for the files

**categories** a list of categories with their own config sections.  Values which can be overridden on a per-category basis are:  **localpath**, **run_script**, and **notify_script** 