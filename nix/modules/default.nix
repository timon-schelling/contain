{ self, ... }:

rec {
  default = guest;
  guest = import ./guest self;
  host = import ./host self;
}
