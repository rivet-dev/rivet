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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDashcube.js
var require_faDashcube = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDashcube.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "dashcube";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f210";
    var svgPathData = "M326.9 104l-216.2 0c-51.1 0-91.2 43.3-91.2 93.5l0 229.5c0 50.5 40.1 85 91.2 85l227.2 0c51.1 0 91.2-34.5 91.2-85l0-427-102.2 104zM154.2 416.5c-17.7 0-32.4-15.1-32.4-32.8l0-142.9c0-17.7 14.7-32.5 32.4-32.5l140.7 0c17.7 0 32 14.8 32 32.5l0 123.5 51.1 52.3-223.8 0 0-.1z";
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
    exports.faDashcube = exports.definition;
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

// src/_entries/faDashcube.js
var import_faDashcube = __toESM(require_faDashcube());
var export_faDashcube = import_faDashcube.definition;
export {
  export_faDashcube as faDashcube
};
