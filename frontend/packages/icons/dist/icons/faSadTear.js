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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceSadTear.js
var require_faFaceSadTear = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceSadTear.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-sad-tear";
    var width = 512;
    var height = 512;
    var aliases = [128546, "sad-tear"];
    var unicode = "f5b4";
    var svgPathData = "M0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zm271 98c5.5-1.3 11.1-2 17-2 24.1 0 45.4 11.8 58.5 30 7.7 10.8 22.7 13.2 33.5 5.5s13.2-22.7 5.5-33.5c-21.7-30.2-57.3-50-97.5-50-12.5 0-24.6 1.9-35.9 5.5 9.8 12.7 16.5 27.9 18.9 44.6zM176 240a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm192-32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM185.4 292.8c-2.4-2.9-5.7-4.8-9.4-4.8s-7 1.9-9.4 4.8c-6.5 7.8-12.6 16.1-18.3 24.6-9 13.4-20.3 30.2-20.3 47.4 0 28.3 21.5 51.2 48 51.2s48-22.9 48-51.2c0-17.2-11.2-34-20.3-47.4-5.7-8.5-11.9-16.7-18.3-24.6z";
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
    exports.faFaceSadTear = exports.definition;
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

// src/_entries/faSadTear.js
var import_faFaceSadTear = __toESM(require_faFaceSadTear());
var export_faSadTear = import_faFaceSadTear.definition;
export {
  export_faSadTear as faSadTear
};
