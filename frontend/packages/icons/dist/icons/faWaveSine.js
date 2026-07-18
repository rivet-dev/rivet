var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWaveSine.js
var require_faWaveSine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWaveSine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wave-sine";
    var width = 640;
    var height = 512;
    var aliases = [8767];
    var unicode = "f899";
    var svgPathData = "M63.4 230.3l0-.1 .2-1c.2-.9 .5-2.4 1-4.3c1-3.8 2.5-9.4 4.6-16.2C73.6 195 80.3 177 90 159.4C110.2 122.8 137.8 96 176 96c29 0 49.1 14.4 67 44.4c19 31.8 31.9 75.4 46.3 124.5c.2 .8 .5 1.6 .7 2.4c13.5 46.2 28.4 97.5 52 137c25.1 42 63 75.6 122 75.6c73.8 0 118.2-53.2 142-96.6c12.3-22.4 20.6-44.5 25.7-60.8c2.6-8.2 4.5-15.1 5.7-20c.6-2.5 1.1-4.4 1.4-5.9c.2-.7 .3-1.3 .4-1.7l.1-.5 0-.2c0 0 0 0 0-.1c0 0 0 0 0 0L608 288l31.4 6.3c3.5-17.3-7.8-34.2-25.1-37.7s-34.2 7.8-37.7 25.1c0 0 0 0 0 0l0 .1-.2 1c-.2 .9-.5 2.4-1 4.3c-1 3.8-2.5 9.4-4.6 16.2c-4.3 13.7-11.1 31.6-20.8 49.2c-20.1 36.6-47.8 63.4-86 63.4c-29 0-49.1-14.4-67-44.4c-19-31.8-31.9-75.4-46.3-124.5c-.2-.8-.5-1.6-.7-2.4c-13.5-46.2-28.4-97.5-52-137C272.9 65.6 235 32 176 32C102.2 32 57.8 85.2 34 128.6C21.7 151 13.4 173 8.2 189.3c-2.6 8.2-4.5 15.1-5.7 20c-.6 2.5-1.1 4.4-1.4 5.9c-.2 .7-.3 1.3-.4 1.7l-.1 .5 0 .2c0 0 0 0 0 .1c0 0 0 0 0 0L32 224 .6 217.7c-3.5 17.3 7.8 34.2 25.1 37.7s34.2-7.8 37.7-25.1z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faWaveSine = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faWaveSine.js
var import_faWaveSine = __toESM(require_faWaveSine());
var export_faWaveSine = import_faWaveSine.definition;
export {
  export_faWaveSine as faWaveSine
};
