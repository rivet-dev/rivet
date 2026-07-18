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

// src/node_modules/@fortawesome/free-solid-svg-icons/faScrewdriver.js
var require_faScrewdriver = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faScrewdriver.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "screwdriver";
    var width = 576;
    var height = 512;
    var aliases = [129691];
    var unicode = "f54a";
    var svgPathData = "M352.1 146.7l0-49.6c0-10.7 5.3-20.7 14.2-26.6L485.2-8.7c6.3-4.2 14.8-3.4 20.2 2l45.4 45.5c5.4 5.4 6.2 13.8 2 20.2L473.6 177.8c-5.9 8.9-15.9 14.2-26.6 14.2l-49.6 0-90.7 90.7c15 33.3 8.9 73.9-18.5 101.3L162.1 510.1c-18.7 18.7-49.1 18.7-67.9 0L34.1 449.9c-18.7-18.7-18.7-49.1 0-67.9L160.1 256c27.4-27.4 67.9-33.6 101.3-18.5l90.7-90.7z";
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
    exports.faScrewdriver = exports.definition;
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

// src/_entries/faScrewdriver.js
var import_faScrewdriver = __toESM(require_faScrewdriver());
var export_faScrewdriver = import_faScrewdriver.definition;
export {
  export_faScrewdriver as faScrewdriver
};
