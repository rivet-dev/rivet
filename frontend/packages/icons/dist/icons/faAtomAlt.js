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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAtomSimple.js
var require_faAtomSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAtomSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "atom-simple";
    var width = 512;
    var height = 512;
    var aliases = ["atom-alt"];
    var unicode = "f5d3";
    var svgPathData = "M100.9 411.1c-1.3-1.3-12.4-16.8 3.3-64c3.5-10.4 8-21.4 13.6-32.9c11.6 14.2 24.3 28.3 38 41.9s27.7 26.4 41.9 38c-11.4 5.6-22.4 10.2-32.8 13.6c-47.2 15.7-62.7 4.6-64 3.3zM153.7 256c13.5-18.3 29.3-36.8 47.4-54.9s36.6-33.9 54.9-47.4c18.3 13.5 36.8 29.3 54.9 47.4s33.9 36.6 47.4 54.9c-13.5 18.3-29.3 36.8-47.4 54.9s-36.6 33.9-54.9 47.4c-18.3-13.5-36.8-29.3-54.9-47.4s-33.9-36.6-47.4-54.9zm44.1-138.1c-14.2 11.6-28.3 24.3-41.9 38s-26.4 27.7-38 41.9c-5.6-11.4-10.2-22.4-13.6-32.8c-15.7-47.2-4.6-62.7-3.3-64s16.8-12.4 64 3.3c10.4 3.5 21.4 8 32.8 13.6zM256 76.8C173.9 27.6 94.9 16.5 55.7 55.7S27.6 173.9 76.8 256C27.6 338.1 16.5 417.1 55.7 456.3s118.2 28.1 200.3-21.1c82.1 49.2 161.1 60.3 200.3 21.1s28.1-118.2-21.1-200.3c49.2-82.1 60.3-161.1 21.1-200.3S338.1 27.6 256 76.8zm58.2 41.1c11.4-5.6 22.4-10.2 32.9-13.6c47.2-15.7 62.7-4.6 64-3.3s12.4 16.8-3.3 64c-3.5 10.4-8 21.4-13.6 32.8c-11.6-14.2-24.3-28.3-38-41.9s-27.7-26.4-41.9-38zm79.9 196.3c5.6 11.4 10.2 22.4 13.6 32.9c15.7 47.2 4.6 62.7 3.3 64s-16.8 12.4-64-3.3c-10.4-3.5-21.4-8-32.9-13.6c14.2-11.6 28.3-24.3 41.9-38s26.4-27.7 38-41.9zM288 256a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faAtomSimple = exports.definition;
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

// src/_entries/faAtomAlt.js
var import_faAtomSimple = __toESM(require_faAtomSimple());
var export_faAtomAlt = import_faAtomSimple.definition;
export {
  export_faAtomAlt as faAtomAlt
};
