from pathlib import Path
import pynvim

APP_DIR = ".local/share/codex/data"

@pynvim.plugin
class TestPlugin(object):

    def __init__(self, nvim):
        self.nvim = nvim

    @pynvim.function('TestFunction', sync=True)
    def testfunction(self, args):
        return 3

    @pynvim.command('TestCommand', nargs='*', range='')
    def testcommand(self, args, range):
        self.nvim.current.line = ('Command with args: {}, range: {}'
                                  .format(args, range))

    @pynvim.autocmd('BufEnter', pattern='*.py', eval='expand("<afile>")', sync=True)
    def on_bufenter(self, filename):
        self.nvim.out_write('testplugin is in ' + filename + '\n')

    @pynvim.autocmd('VimEnter', sync=False)
    def on_open(self):
        app_dir = Path.home() / APP_DIR
        app_dir.mkdir(parents=True, exist_ok=True)
        self.nvim.chdir(str(app_dir))
        self.nvim.out_write(str(app_dir))
        self.nvim.command("e test.md")
        # import time
        # time.sleep(6)
        self.nvim.out_write("CODEX\n")
