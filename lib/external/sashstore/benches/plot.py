#!/usr/bin/env python3

"""
Script that plots benchmark data-visualizations.
"""

import sys
import os
import pandas as pd
import numpy as np
import plotnine as p9
import re

from plotnine import *
from plotnine.data import *
import humanfriendly

import warnings


def plot_scalability(filename, df):
    "Plots a throughput graph for various threads showing the throughput over time"
    benchmark = df.groupby(['benchmark', 'threads', 'write_ratio', 'capacity', 'dist'], as_index=False).agg(
        {'total_ops': 'sum', 'tid': 'count', 'duration': 'max'})
    benchmark['tps'] = (benchmark['total_ops'] /
                        (benchmark['duration'])).fillna(0.0).astype(int)
    benchmark['label'] = benchmark['benchmark'] #+ \
      #  " " + benchmark['write_ratio'].astype(str)
    benchmark['wr'] = "wr=" + benchmark['write_ratio'].astype(str)
    benchmark.to_csv(r'processed.csv', index = False)

    p = ggplot(data=benchmark, mapping=aes(x='threads', y='tps', ymin=0, xmax=12, color='label')) + \
        labs(y="Throughput [Melems/s]", x="# Threads") + \
        theme(legend_position='top', legend_title=element_blank()) + \
        scale_y_continuous(labels=lambda lst: ["{:,.2f}".format(x / 1_000_000) for x in lst]) + \
        geom_point() + \
        geom_line() + \
        facet_grid(["dist", "wr"], scales="free_y")

    p.save("{}-throughput.png".format(filename), dpi=400, width=25, height=20)
    p.save("{}-throughput.pdf".format(filename), dpi=400)

def plot_memory(filename, df):
    "Plots a memory consumption graph for various threads showing the throughput over time"
    benchmark = df.groupby(['benchmark', 'threads', 'write_ratio', 'capacity', 'dist'], as_index=False).agg(
        {'total_ops': 'sum', 'tid': 'count', 'duration': 'max', 'heap_total': 'max'})
    benchmark['heap_mib'] = (benchmark['heap_total'] / (1024*1024*1))
    benchmark['label'] = benchmark['benchmark']
    benchmark['wr'] = "wr=" + benchmark['write_ratio'].astype(str)

    p = ggplot(data=benchmark, mapping=aes(x='threads', y='heap_mib', ymin=0, xmax=12, color='label')) + \
        labs(y="Peak Heap Memory [MiB]", x="# Threads") + \
        theme(legend_position='top', legend_title=element_blank()) + \
        scale_y_continuous(labels=lambda lst: ["{:,.2f}".format(x) for x in lst]) + \
        geom_point() + \
        geom_line() + \
        facet_grid(["dist", "wr"], scales="free_y")

    p.save("{}-memory.png".format(filename), dpi=400, width=25, height=20)
    p.save("{}-memory.pdf".format(filename), dpi=400)



def parse_results(path):
    return pd.read_csv(path)


if __name__ == '__main__':
    warnings.filterwarnings('ignore')
    pd.set_option('display.max_rows', 500)
    pd.set_option('display.max_columns', 500)
    pd.set_option('display.width', 1000)
    pd.set_option('display.expand_frame_repr', True)

    if len(sys.argv) != 2:
        print("Usage: Give path to .csv results file as first argument.")
        sys.exit(1)

    df = parse_results(sys.argv[1])
    print(df)
    plot_scalability(os.path.basename(sys.argv[1]), df)
    plot_memory(os.path.basename(sys.argv[1]), df)
