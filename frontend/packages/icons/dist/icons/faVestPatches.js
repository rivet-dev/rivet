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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVestPatches.js
var require_faVestPatches = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVestPatches.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vest-patches";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e086";
    var svgPathData = "M200 293.9L200 464c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48L0 270.5c0-9.5 2.8-18.7 8.1-26.6l47.9-71.8c5.3-7.9 8.1-17.1 8.1-26.6L64 48C64 21.5 85.5 0 112 0l3.5 0c.3 0 .6 0 1 0 .6 0 1.2 0 1.8 0 18.8 0 34.1 9.7 44.1 18.8 9.3 8.4 28.5 21.2 61.7 21.2s52.4-12.8 61.7-21.2c10-9.1 25.3-18.8 44.1-18.8 .6 0 1.2 0 1.8 0 .3 0 .6 0 1 0L336 0c26.5 0 48 21.5 48 48l0 97.5c0 9.5 2.8 18.7 8.1 26.6l47.9 71.8c5.3 7.9 8.1 17.1 8.1 26.6L448 464c0 26.5-21.5 48-48 48l-104 0c-26.5 0-48-21.5-48-48l0-170.1c0-3.9 .5-7.8 1.4-11.6L303.6 65.4C285.9 77.2 259.8 88 224 88s-61.9-10.8-79.6-22.6l54.2 216.8c1 3.8 1.4 7.7 1.4 11.6zM96 456a40 40 0 1 0 0-80 40 40 0 1 0 0 80zM63.5 255.5c-4.7 4.7-4.7 12.3 0 17L79 288 63.5 303.5c-4.7 4.7-4.7 12.3 0 17s12.3 4.7 17 0L96 305 111.5 320.5c4.7 4.7 12.3 4.7 17 0s4.7-12.3 0-17L113 288 128.5 272.5c4.7-4.7 4.7-12.3 0-17s-12.3-4.7-17 0L96 271 80.5 255.5c-4.7-4.7-12.3-4.7-17 0zM304 280l0 40c0 8.8 7.2 16 16 16l40 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-8 0 0-8c0-13.3-10.7-24-24-24s-24 10.7-24 24z";
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
    exports.faVestPatches = exports.definition;
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

// src/_entries/faVestPatches.js
var import_faVestPatches = __toESM(require_faVestPatches());
var export_faVestPatches = import_faVestPatches.definition;
export {
  export_faVestPatches as faVestPatches
};
