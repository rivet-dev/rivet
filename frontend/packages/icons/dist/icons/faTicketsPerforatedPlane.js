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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTicketsAirline.js
var require_faTicketsAirline = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTicketsAirline.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tickets-airline";
    var width = 640;
    var height = 512;
    var aliases = ["tickets-perforated-plane", "tickets-plane"];
    var unicode = "e29b";
    var svgPathData = "M96 96l0 224c0 35.3 28.7 64 64 64l416 0c35.3 0 64-28.7 64-64l0-48c0-8.8-7.4-15.7-15.7-18.6C605.5 246.9 592 229 592 208s13.5-38.9 32.3-45.4c8.3-2.9 15.7-9.8 15.7-18.6l0-48c0-35.3-28.7-64-64-64L160 32c-35.3 0-64 28.7-64 64zm416 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm0 64a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm16 48a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm-16 80a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM265.3 106.5c-1.7-5.2 2.1-10.5 7.6-10.5L304 96c8.5 0 16.4 4.5 20.7 11.9L364.4 176l40.3 0c16.1 0 31.5 6.4 42.8 17.7c7.9 7.9 7.9 20.7 0 28.5c-11.4 11.4-26.8 17.7-42.8 17.7l-40.3 0-39.7 68.1c-4.3 7.4-12.2 11.9-20.7 11.9l-31.1 0c-5.5 0-9.3-5.3-7.6-10.5L288.4 240l-42.7 0-19.2 25.6c-3 4-7.8 6.4-12.8 6.4L200 272c-5.2 0-9-4.9-7.8-9.9l12.5-50.2c.6-2.5 .6-5.2 0-7.8l-12.5-50.2c-1.3-5 2.6-9.9 7.8-9.9l13.8 0c5 0 9.8 2.4 12.8 6.4L245.8 176l42.7 0-23.2-69.5zM48 120c0-13.3-10.7-24-24-24S0 106.7 0 120L0 360c0 66.3 53.7 120 120 120l400 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-400 0c-39.8 0-72-32.2-72-72l0-240z";
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
    exports.faTicketsAirline = exports.definition;
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

// src/_entries/faTicketsPerforatedPlane.js
var import_faTicketsAirline = __toESM(require_faTicketsAirline());
var export_faTicketsPerforatedPlane = import_faTicketsAirline.definition;
export {
  export_faTicketsPerforatedPlane as faTicketsPerforatedPlane
};
