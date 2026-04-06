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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHoneyPot.js
var require_faHoneyPot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHoneyPot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "honey-pot";
    var width = 448;
    var height = 512;
    var aliases = [127855];
    var unicode = "e418";
    var svgPathData = "M128 32L64 32C46.3 32 32 46.3 32 64s14.3 32 32 32C25.1 125.2 0 171.7 0 224C0 362.8 79.6 441.9 113.3 469.1c9.3 7.5 21.1 10.9 33.1 10.9l155.3 0c12 0 23.7-3.3 33.1-10.9C368.4 441.9 448 362.8 448 224c0-52.3-25.1-98.8-64-128c17.7 0 32-14.3 32-32s-14.3-32-32-32L176 32l0 104c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-104zM64 224l320 0c0 53.7-14.7 96.1-32.9 128L96.9 352C78.7 320.1 64 277.7 64 224z";
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
    exports.faHoneyPot = exports.definition;
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

// src/_entries/faHoneyPot.js
var import_faHoneyPot = __toESM(require_faHoneyPot());
var export_faHoneyPot = import_faHoneyPot.definition;
export {
  export_faHoneyPot as faHoneyPot
};
