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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarShooting.js
var require_faStarShooting = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarShooting.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "star-shooting";
    var width = 512;
    var height = 512;
    var aliases = [127776];
    var unicode = "e036";
    var svgPathData = "M313 41L265 89c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9L279 7c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9zM170.4 157.4c8.8-17.9 34.3-17.9 43.1 0l46.3 94 103.5 15.1c19.7 2.9 27.5 27 13.3 40.9l-74.9 73.2 17.7 103.3c3.4 19.6-17.2 34.6-34.8 25.3l-92.6-48.8L99.3 509.2c-17.6 9.3-38.2-5.7-34.8-25.3L82.2 380.6 7.2 307.4C-7 293.5 .9 269.3 20.5 266.5l103.5-15.1 46.3-94zM505 199c9.4 9.4 9.4 24.6 0 33.9l-48 48c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l48-48c9.4-9.4 24.6-9.4 33.9 0zM505 41L361 185c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9L471 7c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z";
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
    exports.faStarShooting = exports.definition;
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

// src/_entries/faStarShooting.js
var import_faStarShooting = __toESM(require_faStarShooting());
var export_faStarShooting = import_faStarShooting.definition;
export {
  export_faStarShooting as faStarShooting
};
