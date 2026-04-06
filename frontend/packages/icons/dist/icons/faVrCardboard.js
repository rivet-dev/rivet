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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVrCardboard.js
var require_faVrCardboard = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVrCardboard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vr-cardboard";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f729";
    var svgPathData = "M512 96L64 96C28.7 96 0 124.7 0 160L0 352c0 35.3 28.7 64 64 64l117.5 0c17 0 33.3-6.7 45.3-18.7l33.9-33.9c7.2-7.2 17.1-11.3 27.3-11.3s20.1 4.1 27.3 11.3l33.9 33.9c12 12 28.3 18.7 45.3 18.7L512 416c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64zM80 240a64 64 0 1 1 128 0 64 64 0 1 1 -128 0zm352-64a64 64 0 1 1 0 128 64 64 0 1 1 0-128z";
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
    exports.faVrCardboard = exports.definition;
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

// src/_entries/faVrCardboard.js
var import_faVrCardboard = __toESM(require_faVrCardboard());
var export_faVrCardboard = import_faVrCardboard.definition;
export {
  export_faVrCardboard as faVrCardboard
};
