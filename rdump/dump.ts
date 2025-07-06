import * as fs from 'fs';
import * as path from 'path';

async function dumpFiles() {
  const projectRoot = path.resolve(__dirname, '..');
  const dumpFilePath = path.join(__dirname, 'dump.txt');
  const cargoTomlPath = path.join(__dirname, 'Cargo.toml');
  const srcDir = path.join(__dirname, 'src');

  const findRustFiles = (dir: string): string[] => {
    let results: string[] = [];
    const list = fs.readdirSync(dir);
    list.forEach((file) => {
      file = path.join(dir, file);
      const stat = fs.statSync(file);
      if (stat && stat.isDirectory()) {
        results = results.concat(findRustFiles(file));
      } else if (file.endsWith('.rs')) {
        results.push(file);
      }
    });
    return results;
  };

  try {
    const rustFiles = findRustFiles(srcDir);
    const filesToDump = [cargoTomlPath, ...rustFiles];

    let dumpContent = '';

    for (const filePath of filesToDump) {
      const projectRelativePath = path.relative(projectRoot, filePath);
      dumpContent += `// START ${projectRelativePath}\n\n`;
      const fileContent = fs.readFileSync(filePath, 'utf-8');
      dumpContent += fileContent;
      dumpContent += `\n\n// END ${projectRelativePath}\n\n`;
    }

    fs.writeFileSync(dumpFilePath, dumpContent);
    console.log(`Successfully dumped files to ${dumpFilePath}`);
  } catch (error) {
    console.error('Error dumping files:', error);
  }
}

dumpFiles();
