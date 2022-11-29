# bedrock-finder

## How to use

1. Make a 2D pattern in an editor (X = bedrock, O = air, a = ignore - top left is negative x and z, first line is offset):
   ```
   0,0,0
   XOXOX
   XXXOX
   XOXOX
   ```
2. Add more layers by decrementing the y offset (in different files)
3. Run `bedrock-finder pattern $(cat <file>)` for each file and save the output somewhere
4. Run `bedrock-finder <seed> nether:floor/nether:roof/overworld <distance> <y level to scan at (this is where 0,0,0 will be in the pattern)> <scan only at chunk origin: true/false> <pattern>` where pattern is all the outputs from the previous commands concatenated

