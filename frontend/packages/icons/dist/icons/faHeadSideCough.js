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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideCough.js
var require_faHeadSideCough = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideCough.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-cough";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e061";
    var svgPathData = "M96 512l64 0c53 0 96-43 96-96l0-80c0-8.8 7.2-16 16-16l1 0c26 0 47-21 47-47 0-11-3.9-21.6-10.9-30.1L273.3 200C271.4 197.6 270.1 194.7 269.7 191.7 254 83.3 160.7 0 48 0 40.4 0 32.9 .4 25.4 1.1 10.5 2.6 0 15.9 0 30.9L0 480c0 17.7 14.3 32 32 32l64 0zm16-320a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm63.1 237.2l-41.5-3.5c-12.2-1-21.6-11.2-21.6-23.4 0-10.8 7.3-20.2 17.8-22.8l40.4-10.1c19.2-4.8 37.8 9.7 37.8 29.5 0 17.8-15.2 31.8-32.9 30.3zM480 312a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm-40 24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm-64 48a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm128 0a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM480 504a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm-40-72a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faHeadSideCough = exports.definition;
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

// src/_entries/faHeadSideCough.js
var import_faHeadSideCough = __toESM(require_faHeadSideCough());
var export_faHeadSideCough = import_faHeadSideCough.definition;
export {
  export_faHeadSideCough as faHeadSideCough
};
