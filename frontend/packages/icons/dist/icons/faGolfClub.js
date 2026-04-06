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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGolfClub.js
var require_faGolfClub = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGolfClub.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "golf-club";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f451";
    var svgPathData = "M508.6 46.3c7.9-15.8 1.5-35-14.3-42.9s-35-1.5-42.9 14.3L264 392.5l0-.5L42.1 318C21.4 311.1 0 326.6 0 348.4L0 376l48 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L0 408l0 32 48 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L0 472l0 8c0 17.7 14.3 32 32 32l223.3 0c12 .3 23.6-6.3 29.3-17.7l224-448z";
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
    exports.faGolfClub = exports.definition;
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

// src/_entries/faGolfClub.js
var import_faGolfClub = __toESM(require_faGolfClub());
var export_faGolfClub = import_faGolfClub.definition;
export {
  export_faGolfClub as faGolfClub
};
