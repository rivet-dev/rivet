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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandLizard.js
var require_faHandLizard = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandLizard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-lizard";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f258";
    var svgPathData = "M0 112C0 85.5 21.5 64 48 64l238.5 0c36.8 0 71.2 18 92.1 48.2l113.5 164c13 18.7 19.9 41 19.9 63.8l0 76c0 17.7-14.3 32-32 32l-96 0c-17.7 0-32-14.3-32-32l0-13.8-78.1-50.2-161.9 0c-26.5 0-48-21.5-48-48s21.5-48 48-48l128 0c26.5 0 48-21.5 48-48s-21.5-48-48-48L48 160c-26.5 0-48-21.5-48-48z";
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
    exports.faHandLizard = exports.definition;
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

// src/_entries/faHandLizard.js
var import_faHandLizard = __toESM(require_faHandLizard());
var export_faHandLizard = import_faHandLizard.definition;
export {
  export_faHandLizard as faHandLizard
};
