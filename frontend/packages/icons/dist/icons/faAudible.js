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

// src/node_modules/@fortawesome/free-brands-svg-icons/faAudible.js
var require_faAudible = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faAudible.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "audible";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f373";
    var svgPathData = "M640 199.9l0 54-320 200-320-199.9 0-54 320 200 320-200.1zm-194.5 72l47.1-29.4c-37.2-55.8-100.7-92.6-172.7-92.6s-135.5 36.7-172.6 92.4l.3 0c2.5-2.3 5.1-4.5 7.7-6.7 89.7-74.4 219.4-58.1 290.2 36.3zM225.4 290.7c16.9-11.9 36.5-18.7 57.4-18.7 34.4 0 65.2 18.4 86.4 47.6l45.4-28.4c-20.9-29.9-55.6-49.5-94.8-49.5-38.9 0-73.4 19.4-94.4 49zM103.6 161.1C235.4 56.8 421.8 84.7 521.1 223.2l.7 1 48.8-30.4c-53.5-81.7-145.8-135.7-250.7-135.7-103.5 0-196.6 53.5-250.5 135.6 9.9-10.5 22.7-23.5 34.2-32.6z";
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
    exports.faAudible = exports.definition;
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

// src/_entries/faAudible.js
var import_faAudible = __toESM(require_faAudible());
var export_faAudible = import_faAudible.definition;
export {
  export_faAudible as faAudible
};
