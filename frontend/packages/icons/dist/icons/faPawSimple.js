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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPawSimple.js
var require_faPawSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPawSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paw-simple";
    var width = 448;
    var height = 512;
    var aliases = ["paw-alt"];
    var unicode = "f701";
    var svgPathData = "M96 80a48 48 0 1 1 96 0A48 48 0 1 1 96 80zm1.7 206c6.2-2.3 11.8-6.3 15-12.2l40-71.9c14.4-25.9 41.7-42 71.3-42s56.9 16.1 71.3 42l40 71.9c3.2 5.8 8.8 9.9 15 12.2c38.3 14 65.7 50.8 65.7 94c0 55.2-44.8 100-100 100c-21.2 0-40.8-6.6-56.9-17.8c-17.4-12-52.8-12-70.1 0C172.8 473.4 153.2 480 132 480C76.8 480 32 435.2 32 380c0-43.2 27.4-80 65.7-94zM304 32a48 48 0 1 1 0 96 48 48 0 1 1 0-96zm48 160a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM48 144a48 48 0 1 1 0 96 48 48 0 1 1 0-96z";
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
    exports.faPawSimple = exports.definition;
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

// src/_entries/faPawSimple.js
var import_faPawSimple = __toESM(require_faPawSimple());
var export_faPawSimple = import_faPawSimple.definition;
export {
  export_faPawSimple as faPawSimple
};
