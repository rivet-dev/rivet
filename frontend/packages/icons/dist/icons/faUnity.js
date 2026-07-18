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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUnity.js
var require_faUnity = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUnity.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "unity";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e049";
    var svgPathData = "M243.6 91.6l80.1 46.8c2.9 1.6 3 6.2 0 7.8l-95.2 55.6c-2.9 1.7-6.3 1.6-9 0l-95.2-55.6c-2.9-1.6-3-6.3 0-7.8l80.1-46.8 0-91.6-204.4 119.4 0 238.8 78.4-45.8 0-93.6c-.1-3.3 3.8-5.7 6.7-3.9l95.2 55.6c2.9 1.7 4.5 4.7 4.5 7.8l0 111.2c.1 3.3-3.8 5.7-6.7 3.9L98 346.8 19.6 392.6 224 512 428.4 392.6 350 346.8 269.9 393.6c-2.8 1.7-6.8-.5-6.7-3.9l0-111.2c0-3.3 1.8-6.3 4.5-7.8L362.9 215c2.8-1.7 6.8 .5 6.7 3.9l0 93.6 78.4 45.8 0-238.8-204.4-119.4 0 91.6z";
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
    exports.faUnity = exports.definition;
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

// src/_entries/faUnity.js
var import_faUnity = __toESM(require_faUnity());
var export_faUnity = import_faUnity.definition;
export {
  export_faUnity as faUnity
};
