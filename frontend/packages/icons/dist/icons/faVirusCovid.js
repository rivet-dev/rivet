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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVirusCovid.js
var require_faVirusCovid = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVirusCovid.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "virus-covid";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4a8";
    var svgPathData = "M192 24c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-16 0 0 33.6c30.7 4.2 58.8 16.3 82.3 34.1L386.1 92 374.8 80.6c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l56.6 56.6c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-11.3-11.3-23.8 23.8c17.9 23.5 29.9 51.7 34.1 82.3l33.6 0 0-16c0-13.3 10.7-24 24-24s24 10.7 24 24l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-16-33.6 0c-4.2 30.7-16.3 58.8-34.1 82.3l23.8 23.8 11.3-11.3c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-56.6 56.6c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l11.3-11.3-23.8-23.8c-23.5 17.9-51.7 29.9-82.3 34.1l0 33.6 16 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l16 0 0-33.6c-30.7-4.2-58.8-16.3-82.3-34.1l-23.8 23.8 11.3 11.3c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0L46.7 408.7c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l11.3 11.3 23.8-23.8C97.9 338.8 85.8 310.7 81.6 280l-33.6 0 0 16c0 13.3-10.7 24-24 24S0 309.3 0 296l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24l0 16 33.6 0c4.2-30.7 16.3-58.8 34.1-82.3L92 125.9 80.6 137.2c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l56.6-56.6c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9L125.9 92 149.7 115.7c23.5-17.9 51.7-29.9 82.3-34.1l0-33.6-16 0c-13.3 0-24-10.7-24-24zm32 200a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm96 96a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faVirusCovid = exports.definition;
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

// src/_entries/faVirusCovid.js
var import_faVirusCovid = __toESM(require_faVirusCovid());
var export_faVirusCovid = import_faVirusCovid.definition;
export {
  export_faVirusCovid as faVirusCovid
};
