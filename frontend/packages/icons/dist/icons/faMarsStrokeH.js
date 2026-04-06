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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStrokeRight.js
var require_faMarsStrokeRight = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStrokeRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mars-stroke-right";
    var width = 640;
    var height = 512;
    var aliases = [9897, "mars-stroke-h"];
    var unicode = "f22b";
    var svgPathData = "M320.5 256a112 112 0 1 0 -224 0 112 112 0 1 0 224 0zM208.5 80c86.3 0 158.1 62.1 173.1 144.1 1-.1 1.9-.1 2.9-.1l16 0 0-32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 32 61.4 0-22.4-28c-11-13.8-8.8-33.9 5-45s33.9-8.8 45 5l64 80c9.3 11.7 9.3 28.3 0 40l-64 80c-11 13.8-31.2 16-45 5s-16-31.2-5-45l22.4-28-61.4 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-16 0c-1 0-1.9 0-2.9-.1-15 82-86.8 144.1-173.1 144.1-97.2 0-176-78.8-176-176s78.8-176 176-176z";
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
    exports.faMarsStrokeRight = exports.definition;
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

// src/_entries/faMarsStrokeH.js
var import_faMarsStrokeRight = __toESM(require_faMarsStrokeRight());
var export_faMarsStrokeH = import_faMarsStrokeRight.definition;
export {
  export_faMarsStrokeH as faMarsStrokeH
};
