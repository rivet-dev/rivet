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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHelmetSafety.js
var require_faHelmetSafety = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHelmetSafety.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "helmet-safety";
    var width = 576;
    var height = 512;
    var aliases = ["hard-hat", "hat-hard"];
    var unicode = "f807";
    var svgPathData = "M352 264l0-200c0-17.7-14.3-32-32-32l-64 0c-17.7 0-32 14.3-32 32l0 200c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-185.9C90 109.8 32 191.8 32 288l0 64 512 0 0-64c-1-95.2-58.4-177.7-144-209.8L400 264c0 13.3-10.7 24-24 24s-24-10.7-24-24zM40 400c-22.1 0-40 17.9-40 40s17.9 40 40 40l496 0c22.1 0 40-17.9 40-40s-17.9-40-40-40L40 400z";
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
    exports.faHelmetSafety = exports.definition;
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

// src/_entries/faHatHard.js
var import_faHelmetSafety = __toESM(require_faHelmetSafety());
var export_faHatHard = import_faHelmetSafety.definition;
export {
  export_faHatHard as faHatHard
};
