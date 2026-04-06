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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFloppyDisks.js
var require_faFloppyDisks = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFloppyDisks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "floppy-disks";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e183";
    var svgPathData = "M96 56c0-30.9 25.1-56 56-56L370.7 0c14.9 0 29.1 5.9 39.6 16.4l85.3 85.3c10.5 10.5 16.4 24.7 16.4 39.6L512 360c0 30.9-25.1 56-56 56l-304 0c-30.9 0-56-25.1-56-56L96 56zm96 8c-17.7 0-32 14.3-32 32l0 32c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32L192 64zM368 288a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zM48 120l0 256c0 48.6 39.4 88 88 88l256 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-256 0C60.9 512 0 451.1 0 376L0 120c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faFloppyDisks = exports.definition;
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

// src/_entries/faFloppyDisks.js
var import_faFloppyDisks = __toESM(require_faFloppyDisks());
var export_faFloppyDisks = import_faFloppyDisks.definition;
export {
  export_faFloppyDisks as faFloppyDisks
};
