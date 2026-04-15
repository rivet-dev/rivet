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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTarpDroplet.js
var require_faTarpDroplet = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTarpDroplet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tarp-droplet";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e57c";
    var svgPathData = "M256 160c35.3 0 64-26.9 64-60 0-24-33.7-70.1-52.2-93.5-6.1-7.7-17.5-7.7-23.6 0-18.5 23.4-52.2 69.5-52.2 93.5 0 33.1 28.7 60 64 60zM368 96c0 61.9-50.1 112-112 112S144 157.9 144 96c0-11.1 1.6-21.9 4.6-32L64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l277.5 0c17 0 33.3-6.7 45.3-18.7L493.3 322.7c12-12 18.7-28.3 18.7-45.3L512 128c0-35.3-28.7-64-64-64l-84.6 0c3 10.1 4.6 20.9 4.6 32zm85.5 176L336 389.5 336 296c0-13.3 10.7-24 24-24l93.5 0zM96 128a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faTarpDroplet = exports.definition;
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

// src/_entries/faTarpDroplet.js
var import_faTarpDroplet = __toESM(require_faTarpDroplet());
var export_faTarpDroplet = import_faTarpDroplet.definition;
export {
  export_faTarpDroplet as faTarpDroplet
};
