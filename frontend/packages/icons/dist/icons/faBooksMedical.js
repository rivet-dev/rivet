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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBooksMedical.js
var require_faBooksMedical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBooksMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "books-medical";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f7e8";
    var svgPathData = "M64 32C64 14.3 78.3 0 96 0l64 0c17.7 0 32 14.3 32 32l0 64L64 96l0-64zm0 384l128 0 0 64c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-64zM224 32c0-17.7 14.3-32 32-32l64 0c17.7 0 32 14.3 32 32l0 64L224 96l0-64zm0 96l128 0 0 256-128 0 0-256zm0 288l128 0 0 64c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-64zm203.6-19.9L384 232.6l0-89.9 100.4-26.9 66 247.4L427.6 396.1zM476.2 85L384 109.6 384 11l36.9-9.9c16.9-4.6 34.4 5.5 38.9 22.6L476.2 85zM435.8 427l122.8-32.9 16.3 61.1c4.5 17-5.5 34.5-22.5 39.1l-61.4 16.5c-16.9 4.6-34.4-5.5-38.9-22.6L435.8 427zM64 176c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 48 48 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-48 0 0 48c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-48-48 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l48 0 0-48z";
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
    exports.faBooksMedical = exports.definition;
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

// src/_entries/faBooksMedical.js
var import_faBooksMedical = __toESM(require_faBooksMedical());
var export_faBooksMedical = import_faBooksMedical.definition;
export {
  export_faBooksMedical as faBooksMedical
};
