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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStubber.js
var require_faStubber = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStubber.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "stubber";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5c7";
    var svgPathData = "M136.5 294.2l58.8 22.9c9.1-36.8 25.4-61.1 55-61.1 49.4 0 71.4 63.6 142.4 63.6 15.6 0 35.9-2.8 55.3-13.3l0 61.7c0 61.8-50.4 112-112.3 112L0 480 41.8 424 0 368 41.7 312 0 256.1 41.8 200.1 0 144.1 41.8 88 0 32 335.7 32C397.6 32 448 82.3 448 144.1l0 51.3c-9.2 36.3-25.9 60.6-55 60.6-49.6 0-71.6-63.5-142.4-63.5-35.9 0-95.2 14.6-114.1 101.6l0 .1z";
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
    exports.faStubber = exports.definition;
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

// src/_entries/faStubber.js
var import_faStubber = __toESM(require_faStubber());
var export_faStubber = import_faStubber.definition;
export {
  export_faStubber as faStubber
};
