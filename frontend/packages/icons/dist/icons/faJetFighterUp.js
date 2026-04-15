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

// src/node_modules/@fortawesome/free-solid-svg-icons/faJetFighterUp.js
var require_faJetFighterUp = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faJetFighterUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "jet-fighter-up";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e518";
    var svgPathData = "M206.8 47.8C202.3 58.5 200 70 200 81.6l0 100.4-152 114 0-48c0-13.3-10.7-24-24-24S0 234.7 0 248L0 392c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8 152 0 0 54.4-66 52.8c-3.8 3-6 7.6-6 12.5l0 24.3c0 8.8 7.2 16 16 16l88 0 0-40c0-13.3 10.7-24 24-24s24 10.7 24 24l0 40 88 0c8.8 0 16-7.2 16-16l0-24.3c0-4.9-2.2-9.5-6-12.5l-66-52.8 0-54.4 152 0 0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-144c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 48-152-114 0-100.4c0-11.6-2.3-23.1-6.8-33.8l-27.1-65C274.4-26.2 265.7-32 256-32s-18.4 5.8-22.2 14.8l-27.1 65z";
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
    exports.faJetFighterUp = exports.definition;
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

// src/_entries/faJetFighterUp.js
var import_faJetFighterUp = __toESM(require_faJetFighterUp());
var export_faJetFighterUp = import_faJetFighterUp.definition;
export {
  export_faJetFighterUp as faJetFighterUp
};
