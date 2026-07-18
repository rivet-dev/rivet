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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendars.js
var require_faCalendars = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendars.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "calendars";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0d7";
    var svgPathData = "M224 0c-17.7 0-32 14.3-32 32l0 32-48 0c-26.5 0-48 21.5-48 48l0 48 416 0 0-48c0-26.5-21.5-48-48-48l-48 0 0-32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 32-96 0 0-32c0-17.7-14.3-32-32-32zM96 192l0 176c0 26.5 21.5 48 48 48l320 0c26.5 0 48-21.5 48-48l0-176L96 192zm-48-8c0-13.3-10.7-24-24-24s-24 10.7-24 24L0 392c0 66.3 53.7 120 120 120l272 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-272 0c-39.8 0-72-32.2-72-72l0-208z";
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
    exports.faCalendars = exports.definition;
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

// src/_entries/faCalendars.js
var import_faCalendars = __toESM(require_faCalendars());
var export_faCalendars = import_faCalendars.definition;
export {
  export_faCalendars as faCalendars
};
