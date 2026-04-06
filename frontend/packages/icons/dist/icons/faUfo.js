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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUfo.js
var require_faUfo = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUfo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ufo";
    var width = 640;
    var height = 512;
    var aliases = [128760];
    var unicode = "e047";
    var svgPathData = "M320 112c59.2 0 109.6 37.9 128.3 90.7c5 14.2 7.7 29.4 7.7 45.3c0 0-40 40-136 40s-136-40-136-40c0-15.9 2.7-31.1 7.7-45.3c18.7-52.8 69-90.7 128.3-90.7zm0-48c-90.1 0-165.2 64.8-180.9 150.4C55.1 237.5 0 276.2 0 320c0 70.7 143.3 128 320 128s320-57.3 320-128c0-43.8-55.1-82.5-139.1-105.6C485.2 128.8 410.2 64 320 64zm0 288a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM104 328a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm408-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faUfo = exports.definition;
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

// src/_entries/faUfo.js
var import_faUfo = __toESM(require_faUfo());
var export_faUfo = import_faUfo.definition;
export {
  export_faUfo as faUfo
};
