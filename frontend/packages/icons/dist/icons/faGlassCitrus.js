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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassCitrus.js
var require_faGlassCitrus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassCitrus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "glass-citrus";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f869";
    var svgPathData = "M448 144c0-44.2-35.8-80-80-80c-26.2 0-49.4 12.6-64 32l-71.8 0C252 40.1 305.3 0 368 0c79.5 0 144 64.5 144 144c0 78.4-62.6 142.1-140.5 144l6.5-64.6c39.5-4.9 70-38.6 70-79.4zM249.3 224l-.1 1.3-.9-1.3 1 0zM8.3 138.5C14.3 131.8 23 128 32 128l288 0c9 0 17.7 3.8 23.7 10.5s9 15.7 8.1 24.7L322.7 454.4C319.5 487.1 291.9 512 259 512L93 512c-32.9 0-60.4-24.9-63.7-57.6L.2 163.2c-.9-9 2-18 8.1-24.7zM67.4 192l6.4 64 204.5 0 6.4-64L67.4 192z";
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
    exports.faGlassCitrus = exports.definition;
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

// src/_entries/faGlassCitrus.js
var import_faGlassCitrus = __toESM(require_faGlassCitrus());
var export_faGlassCitrus = import_faGlassCitrus.definition;
export {
  export_faGlassCitrus as faGlassCitrus
};
