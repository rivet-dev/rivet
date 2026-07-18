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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUmbrellaBeach.js
var require_faUmbrellaBeach = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUmbrellaBeach.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "umbrella-beach";
    var width = 512;
    var height = 512;
    var aliases = [127958];
    var unicode = "f5ca";
    var svgPathData = "M497.5 341.1c-5.9 16.7-25.3 23-41.1 15.1l-178.2-89.1-1.6 3.2-88.8 177.7 292.2 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l84.2 0 103.2-206.3 1.6-3.2-165.4-82.7c-15.8-7.9-22.4-27.3-12.5-42 45.9-68.6 124.1-113.8 212.9-113.8 141.4 0 256 114.6 256 256 0 29.8-5.1 58.5-14.5 85.1z";
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
    exports.faUmbrellaBeach = exports.definition;
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

// src/_entries/faUmbrellaBeach.js
var import_faUmbrellaBeach = __toESM(require_faUmbrellaBeach());
var export_faUmbrellaBeach = import_faUmbrellaBeach.definition;
export {
  export_faUmbrellaBeach as faUmbrellaBeach
};
