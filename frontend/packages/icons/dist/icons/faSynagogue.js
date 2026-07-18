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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSynagogue.js
var require_faSynagogue = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSynagogue.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "synagogue";
    var width = 576;
    var height = 512;
    var aliases = [128333];
    var unicode = "f69b";
    var svgPathData = "M16 80c0-35.3 28.7-64 64-64s64 28.7 64 64l0 32-128 0 0-32zm0 368l0-288 128 0 126.2-84.2c10.7-7.2 24.8-7.2 35.5 0l126.2 84.2 128 0 0 288c0 35.3-28.7 64-64 64L80 512c-35.3 0-64-28.7-64-64zM560 112l-128 0 0-32c0-35.3 28.7-64 64-64s64 28.7 64 64l0 32zM224 384l0 80 128 0 0-80c0-35.3-28.7-64-64-64s-64 28.7-64 64zm64-152a40 40 0 1 0 0-80 40 40 0 1 0 0 80z";
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
    exports.faSynagogue = exports.definition;
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

// src/_entries/faSynagogue.js
var import_faSynagogue = __toESM(require_faSynagogue());
var export_faSynagogue = import_faSynagogue.definition;
export {
  export_faSynagogue as faSynagogue
};
