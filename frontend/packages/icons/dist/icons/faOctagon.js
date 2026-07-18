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

// src/node_modules/@fortawesome/free-solid-svg-icons/faOctagon.js
var require_faOctagon = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faOctagon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "octagon";
    var width = 512;
    var height = 512;
    var aliases = [128721];
    var unicode = "f306";
    var svgPathData = "M188.6 .1c-17 0-33.3 6.7-45.3 18.7L19.2 143C7.2 155 .5 171.2 .5 188.2l0 135.6c0 17 6.7 33.3 18.7 45.3L143.4 493.2c12 12 28.3 18.7 45.3 18.7l135.6 0c17 0 33.3-6.7 45.3-18.7L493.6 369c12-12 18.7-28.3 18.7-45.3l0-135.6c0-17-6.7-33.3-18.7-45.3L369.5 18.8c-12-12-28.3-18.7-45.3-18.7L188.6 .1z";
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
    exports.faOctagon = exports.definition;
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

// src/_entries/faOctagon.js
var import_faOctagon = __toESM(require_faOctagon());
var export_faOctagon = import_faOctagon.definition;
export {
  export_faOctagon as faOctagon
};
