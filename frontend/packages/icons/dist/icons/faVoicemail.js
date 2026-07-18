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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVoicemail.js
var require_faVoicemail = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVoicemail.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "voicemail";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f897";
    var svgPathData = "M144 160a80 80 0 1 1 0 160 80 80 0 1 1 0-160zM263.8 320c15.3-22.9 24.2-50.4 24.2-80 0-79.5-64.5-144-144-144S0 160.5 0 240 64.5 384 144 384l352 0c79.5 0 144-64.5 144-144S575.5 96 496 96 352 160.5 352 240c0 29.6 8.9 57.1 24.2 80l-112.5 0zM496 160a80 80 0 1 1 0 160 80 80 0 1 1 0-160z";
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
    exports.faVoicemail = exports.definition;
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

// src/_entries/faVoicemail.js
var import_faVoicemail = __toESM(require_faVoicemail());
var export_faVoicemail = import_faVoicemail.definition;
export {
  export_faVoicemail as faVoicemail
};
